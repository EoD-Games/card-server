use phf::phf_map;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use std::sync::Arc;

pub fn get_inv(socket: Arc<Mutex<TcpStream>>) {

}

pub const HANDLERS: phf::Map<&'static str, fn(Arc<Mutex<TcpStream>>)> = phf_map! {
	"get_inv" => get_inv
};