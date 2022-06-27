use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;

use serde_json::json;

extern crate redis;
use redis::Commands;

pub fn listen_redis(cli: &redis::Client) {
    // can't continue without redis connection

    let mut getcon = cli
        .get_connection()
        .expect("Failed to create pub/sub get connection!");

    let mut subcon = cli
        .get_connection()
        .expect("Failed to create pub/sub connection!");
    let mut pubsub = subcon.as_pubsub();

    pubsub
        .psubscribe("__keyspace@0__:VRSCTEST:block:*")
        .expect("Failed to subscribe to channel");
    println!("Started listening to redis subscribe");

    // can continue without discord bot connection
    let discord_url = "127.0.0.1:6666";

    let mut discord_stream = TcpStream::connect(discord_url);
    if discord_stream.is_ok() {
        println!("Discord notification connection successfull!");
    } else {
        println!("Failed to connect to discord bot.");
    }
    // println!

    loop {
        let msg = pubsub.get_message();

        // try to reconnect if connection is closed
        if discord_stream.is_err() {
            discord_stream = TcpStream::connect(discord_url);
        }

        match msg {
            Ok(msg) => {
                let channel_name = msg.get_channel_name();
                // ignore keyspace, get key name
                let (_, key) = channel_name.split_once(":").unwrap();
                println!("Received pub/sub channel: {}, key: {}", channel_name, key);
                
                let block: HashMap<String, String> = getcon.hgetall(key).unwrap();
                println!("block: {:?}", block);

                match &mut discord_stream {
                    Ok(dstream) => {
                        match dstream.write(json!(block).to_string().as_bytes()){
                            Ok(_size) => {
                                println!("Notified discord bot on new message");
                            }, 
                            Err(err) => {
                                println!("Failed to notify discord bot, err: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        println!("Failed to reconnect to discord stream! err: {}", err);
                    }
                }
            }
            Err(err) => {
                println!("Failed to get pub/sub message! err: {}", err);
            }
        }
    }
}
