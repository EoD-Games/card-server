use dotenv::dotenv;
use ezoauth;
use reqwest;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task;
use std::env;
use std::error::Error;
use std::sync::Arc;

mod handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	dotenv().ok();
	let mut addr = "127.0.0.1:".to_string();
	let port = env::args().nth(1).unwrap_or_else(|| "21337".to_string());
	addr.push_str(&port);
	let listener = TcpListener::bind(&addr).await?;
	println!("Listening on {addr}");
	loop {
		let (socket, _) = listener.accept().await?;
		tokio::spawn(async move {
			let sockref = Arc::new(Mutex::new(socket));
			// authenticate
			let config = ezoauth::OAuthConfig {
				auth_url: "https://discord.com/api/oauth2/authorize",
				token_url: "https://discord.com/api/oauth2/token",
				redirect_url: "http://localhost:8696",
				client_id: "1029317200424730680",
				client_secret: &env::var("AAB_OAUTH_SECRET").unwrap(),
				scopes: vec!["identify"]
			};
			let (rx, auth_url) = ezoauth::authenticate(config, "localhost:8696").expect("Failed to authenticate");
			println!("Client should authenticate at {auth_url}");
			let mut sock = sockref.lock().await;
			sock.write_all(json! ({
				"type": "oauth",
				"oauth_url": auth_url
			}).to_string().as_bytes()).await.expect("Failed to send OAuth URL");
			std::mem::drop(sock); // unlock mutex
			let ores = rx.recv().unwrap().expect("No token");
			let token = ores.access_token();
			let res: Value = serde_json::from_str(&reqwest::Client::new().get("https://discord.com/api/users/@me")
				.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
				.send()
				.await.expect("Failed to get user")
				.text()
				.await.expect("Failed to get text")
			).expect("Failed to dejson");
			let uid = res["id"].as_str().unwrap();
			let tag = format!("{}#{}", res["username"].as_str().unwrap(), res["discriminator"].as_str().unwrap());
			println!("Client with id {uid} and username {tag} has joined.");
			let mut sock = sockref.lock().await;
			sock.write_all(json! ({
				"type": "connack"
			}).to_string().as_bytes()).await.expect("Failed to acknowledge connection");
			std::mem::drop(sock);
			// start handling requests
			loop {
				let mut buf = String::new();
				let mut sock = sockref.lock().await;
				sock.read_to_string(&mut buf).await.expect("Failed to read from socket");
				std::mem::drop(sock);
				let res: Value = serde_json::from_str(&buf).expect("Invalid JSON");
				let mtyp = res["type"].as_str().unwrap().to_owned();
				let nref = sockref.clone();
				if handlers::HANDLERS.contains_key(&mtyp) {
					task::spawn(async {
						let mtyp = mtyp;
						handlers::HANDLERS.get(&mtyp).unwrap()(nref)
					});
				} else {
					println!("No handler for message type {mtyp}");
				}
			}
		});
	}
}