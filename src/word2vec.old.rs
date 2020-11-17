use std::path::PathBuf;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::boxed::Box;
use std::time::{Duration, Instant};

enum word2vecError {
    NotImplemented,
    CouldNotOpenFile,
    ErrorReadingFile,
}

struct VocabWord {
    count: u64,
    word: String,
    order_id: usize,
}

struct Vocab {
    vocab: HashMap<String,VocabWord>,
    sorted: Vec<String>,
}

#[derive(Clone)]
struct HuffmanNode<T,U> where {
    node: Vec<Box<HuffmanNode<T,U>>>,
    value: T,
    label: Option<U>,
}

impl HuffmanNode<u64, String> {
    pub fn new(value: u64, label: String) -> HuffmanNode<u64, String> {
        HuffmanNode {
            node: Vec::new(),
            value,
            label: Some(label)
        }
    }

    pub fn combine(a: HuffmanNode<u64, String>, b: HuffmanNode<u64, String>) -> HuffmanNode<u64, String> {
        let value = a.value+b.value; 
        HuffmanNode {
            node: vec![Box::new(a),Box::new(b)],
            value,
            label: None,
        }
    }
}



impl Vocab {
    pub fn new(expected_capacity: usize) -> Vocab {
        assert_ne!(expected_capacity, 0);
        Vocab {
            vocab: HashMap::with_capacity(expected_capacity),
            sorted: Vec::with_capacity(expected_capacity),
        }
    }

    fn add_word_to_vocab(&mut self, word: String) {
        if !self.vocab.contains_key(&word) {
            // if its not in the vocab
            self.vocab.insert(word.clone(),VocabWord{
                count: 1,
                word: word,
                order_id: 0,
            });
        } else {
            // if it is
            if let Some(entry) = self.vocab.get_mut(&word) {
                entry.count+=1;
            }
        }
    }

    fn create_binary_tree(&mut self) -> HuffmanNode<u64, String> {
        let temp_vocab: &HashMap<String,VocabWord> = &self.vocab;
        // get ordered list of counts
        let counts : Vec<u64> = self.sorted.iter().map(|label| temp_vocab.get(label).unwrap().count).collect();
        // convert to leaves, smallest to largest
        let mut leaves: Vec<HuffmanNode<u64, String>> = counts.iter().zip(self.sorted.iter()).map(|(value,label)| HuffmanNode::new(value.clone(), label.clone())).collect();
        assert!(leaves[0].value > leaves[leaves.len()-1].value);
        
        let mut c: HuffmanNode<u64,String>;
        let mut insertion_index: usize = 0 ;
        let mut leaves_length = leaves.len()-1;
        let mut ref_time = Instant::now();
        while leaves.len() > 2 {
            c = HuffmanNode::combine(leaves.remove(leaves_length), leaves.remove(leaves_length-1));
            insertion_index = 0;

            // use binary search here, it'll be faster
            for (index,leaf) in leaves.iter().rev().enumerate() {
                if leaf.value > c.value {
                    insertion_index = leaves_length-index;
                    break;
                }
            }
            leaves.insert(insertion_index, c);

            if leaves.len() % 1000 == 0 {
                println!("{} leaves to go, {:2} msecs/leaf", leaves.len(), (Instant::now()-ref_time).as_secs_f32());
                ref_time=Instant::now();
            }
            leaves_length-=1;

            // leaves.push(c);
            // leaves.sort_by(|a,b| a.value.cmp(&b.value));
        }
        leaves[0].clone()
    }
    
    pub fn learn_vocab_from_train_file(&mut self, file_path: &PathBuf) -> Result<(),word2vecError> {
        if self.vocab.len() == 0 {
            self.add_word_to_vocab(String::from("</s>"));
        }
        let f = match File::open(file_path) {
            Ok(f_pointer) => f_pointer,
            Err(_) => return Err(word2vecError::CouldNotOpenFile),
        };
        let mut reader = BufReader::new(f);
        // By character
        let mut buf = Vec::<u8>::new();
        let mut temp_word = String::new();
        while reader.read_until(b'\n', &mut buf).expect("read_until failed") != 0 {
            let s = String::from_utf8(buf).expect("from_utf8 failed").to_lowercase();
            for c in s.chars() {
                match c {
                    '\r' => continue,
                    ' '|'\t' => {
                        if temp_word.len() > 0 {
                            self.add_word_to_vocab(temp_word);
                            temp_word=String::new();
                        }
                        continue
                    },
                    '\n' => {
                        if temp_word.len() > 0 {
                           self.add_word_to_vocab(temp_word);
                           temp_word=String::new();
                        }
                        self.add_word_to_vocab(String::from("</s>"));
                    },
                    _ => {
                        temp_word.push(c);
                    }
                }
            }
            buf = s.into_bytes();
            buf.clear();
        }

        self.sorted = self.vocab.keys().map(|input| input.clone()).collect();
        let temp_vocab: &HashMap<String,VocabWord> = &self.vocab;
        // create a sorted list for later use
        self.sorted.sort_by(|a,b| temp_vocab.get_key_value(b).unwrap().1.count.cmp(&temp_vocab.get_key_value(a).unwrap().1.count) );

        // write that info to the word structs fro accessability
        for (index,word) in self.sorted.iter().enumerate() {
            if let Some(reference) = self.vocab.get_mut(word) {
                reference.order_id=index;
            }
        }


        Ok(())
        // if !self.vocab.contains_key(word) {

        // }
    
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    #[test]
    fn init_vocab() {
        let vocab = Vocab::new(400);
    }

    #[test]
    fn read_small_file() {
        let mut vocab = Vocab::new(400);
        vocab.learn_vocab_from_train_file(&PathBuf::from(String::from("./test_material/bbc_games_article.txt")));
        println!("read in {} words", vocab.vocab.len());
        for i in  0..5 {
            println!("word: {}, with {} occurances", vocab.sorted[i], vocab.vocab.get_key_value(&vocab.sorted[i]).unwrap().1.count);
        }
        assert_eq!(45,vocab.vocab.get_key_value(&String::from("</s>")).unwrap().1.count);
        assert_eq!(19,vocab.vocab.get_key_value(&String::from("the")).unwrap().1.count);
    }

    #[test]
    fn test_tree_building() {
        let mut vocab = Vocab::new(400);
        // vocab.learn_vocab_from_train_file(&PathBuf::from(String::from("./test_material/bbc_games_article.txt")));
        vocab.learn_vocab_from_train_file(&PathBuf::from(String::from("./test_material/text8")));
        println!("read in {} words", vocab.vocab.len());
        for i in  0..5 {
            println!("word: {}, with {} occurances", vocab.sorted[i], vocab.vocab.get_key_value(&vocab.sorted[i]).unwrap().1.count);
        }
        let tree = vocab.create_binary_tree();
        println!("tree score {}",tree.value);
    }

    // #[test]
    // fn speed_benchmark() {
    //     let mut vocab = Vocab::new(400);
    //     vocab.learn_vocab_from_train_file(&PathBuf::from(String::from("./test_material/text8")));
    //     println!("read in {} words", vocab.vocab.len());
    //     for i in  0..5 {
    //         println!("word: {}, with {} occurances", vocab.sorted[i], vocab.vocab.get_key_value(&vocab.sorted[i]).unwrap().1.count);
    //     }
    //     // assert_eq!(45,vocab.vocab.get_key_value(&String::from("</s>")).unwrap().1.count);
    //     // assert_eq!(19,vocab.vocab.get_key_value(&String::from("the")).unwrap().1.count);
    // }
}