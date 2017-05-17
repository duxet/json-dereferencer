#[macro_use]
extern crate clap;
extern crate glob;
extern crate serde;
extern crate serde_json;

use clap::App;
use glob::glob;
use std::fs::{canonicalize, File};
use std::io::prelude::*;
use std::path::PathBuf;
use serde_json::{Value};
use serde_json::map::Map;
use serde_json::Value::{Array, Object};

fn main() {
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let input_dir = matches.value_of("input_dir").unwrap();
    let input_dir = get_absolute_path(input_dir);

    let output_dir = matches.value_of("output_dir").unwrap();
    let output_dir = get_absolute_path(output_dir);

    let glob_pattern = input_dir.to_str().unwrap().to_owned() + "/**/*.json";

    for entry in glob(glob_pattern.as_str()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => process_file(&input_dir, &output_dir, &path),
            Err(e) => println!("{:?}", e),
        }
    }
}

fn get_absolute_path(path: &str) -> PathBuf {
    let path_buf = PathBuf::from(path);

    canonicalize(path_buf).expect("Failed to get absolute path")
}

fn load_json(path :&PathBuf) -> Value {
    println!("Processing file: {}", path.to_string_lossy());
    let mut file = File::open(path).expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");

    serde_json::from_str(&contents).unwrap()
}

fn process_file(input_dir :&PathBuf, output_dir :&PathBuf, path :&PathBuf) {
    let data = load_json(&path);

    let mut file_dir = path.clone();
    file_dir.set_file_name("");

    let data = process_json(&file_dir, &data);

    let serialized = serde_json::to_string(&data).expect("Failed to serialize");

    let output_path = path.strip_prefix(input_dir).expect("Failed to get output path");
    let output_path = output_dir.to_str().unwrap().to_owned() + "/" + output_path.to_str().unwrap();

    println!("{:?}", output_path);

    let mut output_file = File::create(output_path).expect("Failed to create file");
    output_file.write_all(serialized.as_bytes()).expect("Failed to write file");
}

fn process_json(path :&PathBuf, json :&Value) -> Value {
    match *json {
        Array(ref array) => process_array(path, array),
        Object(ref object) => process_object(path, object),
        _ => { panic!("invalid json file"); }
    }
}

fn process_object(path :&PathBuf, object :&Map<String, Value>) -> Value {
    let mut result = Map::new();

    if object.contains_key("$ref") {
        let value = object.get("$ref").expect("");
        return process_key_and_value(path, &String::from("$ref"), &value);
    }

    for (key, value) in object.iter() {
        let key = key.clone();
        let value = value.clone();

        let value = process_key_and_value(path, &key, &value);

        result.insert(key, value);
    }

    Object(result)
}

fn process_array(path :&PathBuf, array :&Vec<Value>) -> Value {
    let mut result = Vec::new();

    for value in array.iter() {
        let value = process_json(path, value);
        result.push(value);
    }

    Array(result)
}

fn process_key_and_value(path :&PathBuf, key :&String, value :&Value) -> Value {
    match *value {
        Array(ref array) => return process_array(path, array),
        Object(ref object) => return process_object(path, object),
        Value::String(ref string) => return process_string(path, key, string, value),
        _ => {}
    }

    value.clone()
}

fn process_string(path :&PathBuf, key: &String, string: &String, value :&Value) -> Value {
    if key.to_string() == "$ref" {
        let path = path.join(string);
        return load_json(&path);
    }

    value.clone()
}
