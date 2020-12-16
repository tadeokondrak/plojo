#[macro_use]
extern crate lazy_static;
use itertools::Itertools;
use std::fs::File;
use std::io::{BufRead, BufReader, LineWriter, Write};

mod frequency;
mod parsed;
mod processor;
mod raw;

use frequency::FrequencyAnalyzer;
use processor::Processor;

const CHUNK_SIZE: usize = 1000;

fn main() {
    analyze_frequency("log_parsed.txt");

    // to prevent unused code warnings
    if false {
        read_raw_and_parse("log_raw.txt", "log_parsed.txt");
    }
}

/// Reads a raw log file and parses it into another file
fn read_raw_and_parse(raw_file: &str, out_file: &str) {
    println!("Parsing raw file (this may take a few seconds)...");
    let file = File::open(raw_file).expect("File not found");
    let reader = BufReader::new(file);

    let out_file = File::create(out_file).expect("Unable to create output log file");
    let mut out_file = LineWriter::new(out_file);

    let mut i = 1;
    for lines in &reader.lines().chunks(CHUNK_SIZE) {
        let lines = lines.map(|x| x.unwrap()).collect::<Vec<_>>();

        for line in lines {
            match raw::parse_raw(&line) {
                Ok(parsed) => {
                    let result = serde_json::to_string(&parsed).unwrap();
                    out_file
                        .write_all(result.as_bytes())
                        .expect("Unable to write line");
                    out_file.write_all(b"\n").unwrap();
                }
                Err(e) => {
                    eprintln!("WARNING: {}. Could not parse line {}", e, line);
                }
            }
        }

        if i % 10 == 0 {
            println!("Read {} lines...", i * CHUNK_SIZE);
        }
        i += 1;
    }
    println!("Done!");
}

fn analyze_frequency(file: &str) {
    let file = File::open(file).expect("File not found");
    let reader = BufReader::new(file);
    let mut freq = FrequencyAnalyzer::new();

    for lines in &reader.lines().chunks(CHUNK_SIZE) {
        let lines = lines.map(|x| x.unwrap()).collect::<Vec<_>>();

        for line in lines {
            let entry = serde_json::from_str(&line).expect("Invalid serialized data");
            freq.process(entry);
        }
    }

    let grams_1 = freq.grams_1(2);
    println!("Number of n grams: {}", grams_1.len());
    println!("{:?}", &grams_1[..50]);
}
