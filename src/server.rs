use crate::word2vec;
use std::path::PathBuf;

use warp::Filter;
// use serde_derive::{Deserialize, Serialize};
use serde::{Deserialize, Serialize};
use crossbeam::channel::{
    unbounded,
    Sender,
    Receiver,
    SendError, 
    RecvError,
    TryIter,
    TryRecvError};

// For async
use futures;
use futures::{
    task::{Poll,Context},
    prelude::Future,
};
use core::pin::Pin;
use std::{
    task::Waker,
    sync::{
        Arc, 
        Mutex
    },
};
use futures::StreamExt;
use std::boxed::Box;

pub struct Server {
    model: word2vec::Model,
}

#[derive(Deserialize, Serialize)]
struct ConvertPayload {
    words: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct ConvertResponse {
    words: Vec<Vec<f32>>,
}

#[derive(Deserialize, Serialize)]
struct TestResponse {
    data: String,
}

#[derive(Clone, Debug)]
enum ThreadComm {
    Word2Vec(String),
    WordVec(Option<Vec<f32>>),
    Exit,
}

#[derive(Clone)]
struct Comm<T> {
    sender:Sender<T>,
    receiver:Receiver<T>,
}

impl<T> Comm<T> {
    fn new() -> (Comm<T>,Comm<T>) {
        let (s_a, r_b) = unbounded();
        let (s_b, r_a) = unbounded();

        (
            Comm{
                sender:s_a,
                receiver:r_a,
            },
            Comm{
                sender:s_b,
                receiver:r_b,
            }
        )
    }

    fn send(&self, item: T) -> Result<(),SendError<T>> {
        self.sender.send(item)
    }
    fn recv(&self) -> Result<T,RecvError> {
        self.receiver.recv()
    }
    fn try_iter(&self) -> TryIter<'_,T> {
        self.receiver.try_iter()
    }

    fn try_recv(&self) -> Result<T,TryRecvError> {
        self.receiver.try_recv()
    }
}

// async receiver
struct CommRecvStream<T>{
    state: Arc<Mutex<CommStreamState<T>>>,
}

struct CommStreamState<T> {
    comm: Comm<T>, 
    waker: Option<Waker>,
}

impl<T> futures::stream::Stream for CommRecvStream<T> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>,cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut self_state = self.state.lock().unwrap();
        
        match self_state.comm.try_recv() {
            // we have a value ready!
            Ok(value) => Poll::Ready(Some(value)),
            Err(reason) => {
                match reason {
                    //no message there
                    TryRecvError::Empty =>{ 
                        self_state.waker = Some(cx.waker().clone());
                        Poll::Pending
                    },
                    // line gone dead
                    TryRecvError::Disconnected => Poll::Ready(None),
                }
            }
        }
    }
}

impl<T:std::clone::Clone> CommRecvStream<T> {
    fn new(comm: &Comm<T>) -> CommRecvStream<T> {
        let state = Arc::new(Mutex::new(CommStreamState{
            comm: (*comm).clone(),
            waker: None,
        }));
        CommRecvStream{
            state,
        }

    }
}

/*
    // async Sender
    struct CommSendStream<T>{
        state: Arc<Mutex<CommStreamState<T>>>,
    }


    impl<T> futures::stream::Stream for CommSendStream<T> {
        type Item = T;
        fn poll_next(self: Pin<&mut Self>,cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut self_state = self.state.lock().unwrap();
            
            match self_state.comm.try_recv() {
                // we have a value ready!
                Ok(value) => Poll::Ready(Some(value)),
                Err(reason) => {
                    match reason {
                        //no message there
                        TryRecvError::Empty =>{ 
                            self_state.waker = Some(cx.waker().clone());
                            Poll::Pending
                        },
                        // line gone dead
                        TryRecvError::Disconnected => Poll::Ready(None),
                    }
                }
            }
        }
    }

    impl<T:std::clone::Clone> CommSendStream<T> {
        fn new(comm: &Comm<T>) -> CommSendStream<T> {
            let state = Arc::new(Mutex::new(CommStreamState{
                comm: (*comm).clone(),
                waker: None,
            }));
            CommSendStream{
                state,
            }

        }
    }



    impl<T:std::clone::Clone> CommFuture<T> {
        fn new(comm: &Comm<T>) -> CommFuture<T> {
            let state = Arc::new(Mutex::new(CommFutureState{
                comm: (*comm).clone(),
                waker: None,
            }));
            CommFuture{
                state,
            }

        }
    }
*/


impl Server{
    pub fn new(model_path: PathBuf) -> Option<Server> {
        let model = match word2vec::Model::new(model_path) {
            Ok(model_str) => model_str,
            Err(reason) => {
                println!("{:?}",reason);
                return None;
            }, 
        };
        Some(Server {
            model,
        })
    }

    pub async fn begin(&self) {
        let (comm_a,comm_b):(Comm<ThreadComm>,Comm<ThreadComm>) = Comm::new();
        let fut1 = self.infer(comm_a);
        let fut2 = self.serve(comm_b);
        futures::join!(fut1, fut2);
    }

    async fn infer(&self, comm:Comm<ThreadComm>) {
        println!("starting inference server");
        let mut comm_stream = CommRecvStream::new(&comm);
        while let Some(message) = comm_stream.next().await {
            println!("Then I got: {:?}", message);
            // for message in comm.recv() {
            match message {
                ThreadComm::Word2Vec(word) => {
                    println!("Got message");
                    let return_message = match self.model.word2vec(&word) {
                        Some(vector) => {
                            Some((*vector).clone())
                        }
                        None => {
                            None
                        }
                    };
                    if let Err(reason) = comm.send(ThreadComm::WordVec(return_message)) {
                        println!("Warning, could not send message because:\n\t{}",reason);
                    }
                }
                ThreadComm::Exit => {
                    break;
                }
                _ => {
                    // Do nothing for anothing else
                },
                // ThreadComm::wordVec(vector)

            }
            // }
        }

        println!("Exiting inference server");
    }

    async fn serve(&self, comm:Comm<ThreadComm>) {
        println!("starting web server");
        let boxed_comm_stream = Box::new(CommRecvStream::new(&comm));
        let convert = warp::get()
            .and(warp::path("convert"))
            // Only accept bodies smaller than 16kb...
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .and_then(move |payload: ConvertPayload| async {
                let mut response_vector: Vec<Vec<f32>> = Vec::with_capacity(payload.words.len());
                println!("I got: {:?}",payload.words);
                for word in payload.words.iter() {
                    let s: String = (*word).clone();
                    print!("Converting |{}| ", s);
                    if let Err(reason) = comm.send(ThreadComm::Word2Vec(s)) {
                        println!("I errored be:\n\t{}",reason);
                    }
                    // need to wrap this in an async function too
                    println!("I got to here");
                    if let ThreadComm::WordVec(vec_response_opt) = (*boxed_comm_stream).next().await.unwrap() {
                    // if let ThreadComm::WordVec(vec_response_opt) = comm.recv().unwrap() {
                        if let Some(vec_response) = vec_response_opt {
                            response_vector.push(vec_response);
                        }
                    }
                }
                // warp::reply::
                let a :std::result::Result<warp::reply::Json, _> = Ok(warp::reply::json(&ConvertResponse {
                    words: response_vector,
                }));
                a
            });

        warp::serve(convert).run(([127, 0, 0, 1], 3030)).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[ignore]
    fn init_server() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin");
        let server = Server::new(short_model_path); 
    }

    #[tokio::test]
    async fn run_server() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin");
        let server = Server::new(short_model_path).unwrap(); 
        server.begin().await;
    }


}
