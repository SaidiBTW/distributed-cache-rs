extern crate cache;

use cache::{client::Client, response::Response};

fn main() {
    let key = b"Hello123";
    let value = b"Hello1234Value";

    let missing_key = b"Missing123";

    let mut client = Client::connect("127.0.0.1:7878").expect("Error connecting to server");

    println!("Successfully connected to server");
    // Test the set command;

    match client.set(key, value) {
        Ok(Response::Ok(_)) => println!("Set successful"),
        Ok(Response::Err(error)) => {
            println!(
                "Error from server: Corresponding reads may fail {:#?}",
                error
            )
        }
        _ => println!("Unknown matching arm"),
    }

    match client.get(key) {
        Ok(Response::Ok(response)) => println!(
            "Response from server {:#?}",
            String::from_utf8_lossy(&response)
        ),
        Ok(Response::Err(error)) => println!("Error from server {:#?}", error),
        Ok(Response::NotFound) => println!(
            "Did not find key '{}' in server",
            String::from_utf8_lossy(key)
        ),
        Err(err) => println!("Error {:#?}", err),
    }

    //For missing key
    match client.get(missing_key) {
        Ok(Response::Ok(response)) => println!(
            "Response from server {:#?}",
            String::from_utf8_lossy(&response)
        ),
        Ok(Response::Err(error)) => println!("Error from server {:#?}", error),
        Ok(Response::NotFound) => println!(
            "Did not find key '{}' in server",
            String::from_utf8_lossy(missing_key)
        ),
        Err(err) => println!("Error {:#?}", err),
    }

    //Delete Existing key
    match client.delete(key) {
        Ok(Response::Ok(_)) => println!("Deleted value from server"),
        Ok(Response::NotFound) => println!("Key not found in server"),
        Ok(Response::Err(e)) => println!("Error deleting key from server {}", e),
        Err(e) => eprintln!("Err {:#?}", e),
    }

    //Check Whether key still exists
    match client.get(key) {
        Ok(Response::Ok(response)) => println!(
            "Response from server {:#?}",
            String::from_utf8_lossy(&response)
        ),
        Ok(Response::Err(error)) => println!("Error from server {:#?}", error),
        Ok(Response::NotFound) => println!(
            "Did not find key '{}' in server",
            String::from_utf8_lossy(key)
        ),
        Err(err) => println!("Error {:#?}", err),
    }

    //Delete Non Existing key
    match client.delete(missing_key) {
        Ok(Response::Ok(_)) => println!("Deleted key from server"),
        Ok(Response::NotFound) => println!("Key not found in server"),
        Ok(Response::Err(e)) => println!("Error deleting key from server {}", e),
        Err(e) => eprintln!("Err {:#?}", e),
    }
}
