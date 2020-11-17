use byteorder::{ByteOrder, LittleEndian};
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use std::collections::HashMap;


pub struct Model {
    total_words: usize,
    size: usize,
    lookup: HashMap<String,Vec<f32>>
}

#[derive(Debug)]
pub enum W2VError {
    NotImplemented,
    NoFileAtPath,
    CouldNotOpenFile,
    ReadError(usize),
    UnexpectedEoF,
}

#[derive(Debug)]
enum ReadMode {
    Word,
    Vector,
}

impl Model {
    pub fn new(model_path: PathBuf) -> Result<Model, W2VError> {
        if !model_path.exists() {
            return Err(W2VError::NoFileAtPath);
        }
        // read in binary file
        let f = match fs::File::open(model_path) {
            Ok(pointer) => pointer,
            Err(_) => return Err(W2VError::CouldNotOpenFile),
        };
        let mut reader = BufReader::with_capacity(100000000, f);
        let mut first_line: String = String::new();
        if reader.read_line(&mut first_line).unwrap() == 0 {
            return Err(W2VError::ReadError(0));
        }

        let items: Vec<&str> = first_line.split_ascii_whitespace().collect();
        let total_words: usize = items[0].parse::<usize>().unwrap();
        println!("words: {}", total_words);
        let size: usize = items[1].parse::<usize>().unwrap();
        println!("size: {}", size);

        // let mut words: Vec<String> = Vec::with_capacity(total_words);
        // let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(total_words);
        let mut lookup: HashMap<String,Vec<f32>> = HashMap::with_capacity(total_words); 
        let mut mode: ReadMode = ReadMode::Word;
        // let mut current_word_bytes: Vec<u8> = Vec::with_capacity(40);
        let mut current_vector: Vec<f32> = Vec::with_capacity(size);
        let mut current_value: f32;
        let mut current_value_byte_buffer: Vec<u8> = Vec::with_capacity(size);
        let mut current_word: String = String::with_capacity(50);
        // let mut current_byte: usize = 0;
        print!("Building Model... ");
        for byte_opt in reader.bytes() {
            match byte_opt {
                Ok(byte) => {
                    match mode {
                        ReadMode::Word => match byte {
                            b' ' => {
                                // current_word = String::from_utf8(current_word_bytes.clone()).unwrap();
                                // current_word_bytes.clear();
                                mode = ReadMode::Vector;
                            }
                            b'\n' => { // ignore \n's
                            }
                            _ => {
                                // current_word_bytes.push(byte);
                                current_word.push(byte as char);
                            },
                        },
                        ReadMode::Vector => {
                            match byte {
                                _ => {
                                    current_value_byte_buffer.push(byte);
                                    if current_value_byte_buffer.len() == 4 {
                                        current_value =
                                            LittleEndian::read_f32(&current_value_byte_buffer);
                                        current_vector.push(current_value);
                                        current_value_byte_buffer.clear();
                                        if current_vector.len() == size {
                                            lookup.insert(current_word.clone(), current_vector.clone());
                                            current_word.clear();
                                            current_vector.clear();
                                            mode = ReadMode::Word;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("reached end of file");
                    break;
                    // EoF in this case
                }
            }
        }
        println!("Done\n");
        Ok(Model {
            total_words,
            size,
            lookup,
        })
    }

    pub fn word2vec(&self, word: &String) -> Option<&Vec<f32>> {
        println!("Doing lookup");
        if self.lookup.contains_key(word) {
            self.lookup.get(word)
        } else {
            None
        }
    }

    // returns closest word to the given vec, and its error vec
    pub fn vec2word(&self, ref_vec: &Vec<f32>) -> SortedCosines {
        let mut cosines: HashMap<String,f32> = HashMap::with_capacity(self.lookup.len());
        for (key,value) in self.lookup.iter() {
            cosines.insert(key.clone(), Self::cosine(ref_vec,value));
        }

        // This could be heavily multi-threaded
        let mut keys : Vec<String> = cosines.keys().map(|input| (*input).clone()).collect();
        keys.sort_by(|a,b| (*cosines.get(b).unwrap()).partial_cmp(cosines.get(a).unwrap()).unwrap() );

        SortedCosines {
            cosines,
            keys,
        }
    }

    pub fn get_cosines(&self, word: &String) -> Option<HashMap<String,f32>> {
        let mut return_map: HashMap<String,f32> = HashMap::with_capacity(self.lookup.len());
        let ref_vec: Vec<f32> = match self.word2vec(word) {
            Some(val) => val.clone(),
            None => return None,
        };
        for (key,value) in self.lookup.iter() {
            return_map.insert(key.clone(), Self::cosine(&ref_vec,value));
        }
        Some(return_map)
    }

    pub fn get_sorted_cosines(&self, word: &String) -> Option<SortedCosines> {
        if let Some(cosines) = self.get_cosines(word) {
            // This could be heavily multi-threaded
            let mut keys : Vec<String> = cosines.keys().map(|input| (*input).clone()).collect();
            keys.sort_by(|a,b| (*cosines.get(b).unwrap()).partial_cmp(cosines.get(a).unwrap()).unwrap() );
            
            return Some(SortedCosines {
                cosines,
                keys,
            });
        } else {
            println!("No cosine result");
            return None
        }
    }

    pub fn get_cosine(&self, worda: String, wordb: String) -> Option<f32> {
        if self.lookup.contains_key(&worda) && self.lookup.contains_key(&wordb) {
            Some(self.get_cosine_unchecked(worda,wordb))
        } else {
            None
        }
    }

    fn get_cosine_unchecked(&self, worda: String, wordb: String) -> f32 {
        Self::cosine(self.lookup.get(&worda).unwrap(),self.lookup.get(&worda).unwrap())
    }

    fn cosine(vecA: &Vec<f32>,vecB: &Vec<f32>) -> f32 {
        let mut sum: f32 = 0.0;
        let mut normA: f32 = 0.0;
        let mut normB: f32 = 0.0;
        for (valA,valB) in vecA.iter().zip(vecB) {
            sum += valA*valB;
            normA += valA.powi(2);
            normB += valB.powi(2);
        }
        normA = normA.powf(0.5);
        normB = normB.powf(0.5);

        sum/(normA*normB)
    }
}

pub struct Vec2wordResult {
    word: String,
    cosine: f32,
}

pub struct SortedCosines {
    cosines: HashMap<String,f32>,
    keys   : Vec<String>,
}

impl SortedCosines {
    pub fn get_nth_top(&self, n: usize) -> (std::string::String, f32) {
        let key = self.keys[n].clone();
        let res = (*self.cosines.get(&key).unwrap()).clone();
        (key,res)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;
    #[test]
    fn t01_init_model_false_path() -> Result<(), ()> {
        match Model::new(PathBuf::from("/tmp/nothing")) {
            Err(_) => return Ok(()),
            Ok(_) => return Err(()),
        }
    }

    #[test]
    fn t02_init_model_small() -> Result<(), ()> {
        if let Ok(_model) = Model::new(PathBuf::from("./test_material/vectors.bin")) {
            // if let Ok(_model) = Model::new(PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin")) {
            return Ok(());
        } else {
            return Err(());
        }
    }

    #[test]
    fn t03_do_lookups_small() -> Result<(), String> {
        if let Ok(model) = Model::new(PathBuf::from("./test_material/vectors.bin")) {
            let words = vec!["the","one","in"];
            let mut sum: f32 = 0.0;
            for word in words.iter() {
                match model.word2vec(&String::from(*word)) {
                    Some(vector) => {
                        let mut local_sum: f32 = 0.0;
                        for val in vector {
                            local_sum += val;
                        }
                        sum += local_sum/(vector.len() as f32);
                    }
                    None => {
                        return Err(format!("Could not find {}",word));
                    }
                }
            }
            sum = sum/(words.len() as f32);
            println!("sum: {}",sum);
            return Ok(());
        } else {
            return Err("Could not create model".to_string());
        }
    }

    #[test]
    fn t04_init_model_big() -> Result<(), ()> {
        let start_time = Instant::now();
        if let Ok(_model) = Model::new(PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin")) {
            let finish_time = Instant::now();
            println!("load time: {}",(finish_time-start_time).as_secs());
            return Ok(());
        } else {
            return Err(());
        }
    }

    #[test]
    #[ignore]
    fn t05_do_lookups_big() -> Result<(), String> {
        if let Ok(model) = Model::new(PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin")) {
            let words = vec!["the","one","in"];
            let mut sum: f32 = 0.0;
            for word in words.iter() {
                match model.word2vec(&String::from(*word)) {
                    Some(vector) => {
                        let mut local_sum: f32 = 0.0;
                        for val in vector {
                            local_sum += val;
                        }
                        sum += local_sum/(vector.len() as f32);
                    }
                    None => {
                        return Err(format!("Could not find {}",word));
                    }
                }
            }
            sum = sum/(words.len() as f32);
            println!("sum: {}",sum);
            return Ok(());
        } else {
            return Err("Could not create model".to_string());
        }
    }

    #[test]
    fn t06_get_top5_cosine() -> Result<(), String> {
        if let Ok(model) = Model::new(PathBuf::from("./test_material/vectors.bin")) {
            let words : Vec<String> = vec!["italy","france","paris","rome"].iter().map(|input| (*input).to_string()).collect();
            for word_i in 0..words.len() {
                if let Some(result) = model.get_sorted_cosines(&words[word_i]) {
                    println!("\nMatching against {}",words[word_i]);
                    // print top 10
                    for i in 1..4 {
                        let nth = result.get_nth_top(i);
                        println!("{}-> {} - {},",i,nth.0,nth.1);
                    }
                } else {
                    println!("No match for {}",words[word_i])
                }
            }
            return Ok(());
        } else {
            return Err("Could not create model".to_string());
        }
    }

    #[test]
    fn t07_vector_word_maths()-> Result<(), String> {
        if let Ok(model) = Model::new(PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin")) {
            let paris_vec = model.word2vec(&"king".to_string()).unwrap();
            let france_vec = model.word2vec(&"man".to_string()).unwrap();
            let italy_vec = model.word2vec(&"woman".to_string()).unwrap();

            let new_vec = add_vec(&subtract_vec(&paris_vec, &france_vec),&italy_vec);

            let res = model.vec2word(&new_vec);

            let rome_vec = model.word2vec(&"queen".to_string()).unwrap();

            println!("ideal -> queen - {},",Model::cosine(&rome_vec, &new_vec));


            for i in 0..20 {
                let nth = res.get_nth_top(i);
                println!("{}-> {} - {},",i,nth.0,nth.1);
            }

            return Ok(());
        } else {
            return Err("Could not create model".to_string());
        }
    }

    fn subtract_vec(a:&Vec<f32>,b:&Vec<f32>) -> Vec<f32> {
        a.iter().zip(b.iter()).map(|(a,b)|a-b).collect()
    }

    fn add_vec(a:&Vec<f32>,b:&Vec<f32>) -> Vec<f32> {
        a.iter().zip(b.iter()).map(|(a,b)|a+b).collect()
    }


}
