use async_std::{
    io::{ReadExt, WriteExt},
    net::TcpStream,
};
use futures::AsyncRead;
use std::time::Duration;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Request {
    pub method: String,
    pub params: serde_json::Value,
    pub id: String,
}
impl Request {
    pub fn new(method: String, params: serde_json::Value) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Request { method, params, id }
    }
    pub fn new_with_id(method: String, params: serde_json::Value, id: String) -> Self {
        Request { method, params, id }
    }
    pub async fn send(
        &self,
        stream: &mut async_std::net::TcpStream,
        timeout: Option<Duration>,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let request = serde_json::to_string(&self)?;
        stream.write_all(request.as_bytes()).await?;
        // println!("sent request {:?}", request);
        let main_fut = async {
            let mut buf = [0; 4096];
            let mut msg = String::new();
            loop {
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    break Err("stream closed".into());
                }
                let buf = &buf[..n];
                msg += &String::from_utf8_lossy(buf);
                let res = serde_json::from_str(&msg);
                if res.is_ok() {
                    let response: Response = res.unwrap();
                    if response.id != self.id {
                        msg.clear();
                        continue;
                    }
                    break Ok(response);
                } else {
                    continue;
                }
            }
        };

        if let Some(timeout) = timeout {
            async_std::future::timeout(timeout, main_fut).await?
        } else {
            main_fut.await
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum RpcErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RpcError {
    pub message: String,
    pub code: RpcErrorCode,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Response {
    pub result: serde_json::Value,
    pub error: Option<RpcError>,
    id: String,
}
impl Response {
    pub fn new(result: serde_json::Value, error: Option<RpcError>, id: String) -> Self {
        Response { result, error, id }
    }
    pub async fn send(
        &self,
        stream: &mut async_std::net::TcpStream,
        timeout: Option<Duration>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = serde_json::to_string(&self)?;
        let write_fut = stream.write_all(response.as_bytes());
        if let Some(timeout) = timeout {
            async_std::future::timeout(timeout, write_fut).await??;
        } else {
            write_fut.await?;
        }
        Ok(())
    }
}

pub trait Handler {
    fn handle(
        &mut self,
        request: Request,
    ) -> impl std::future::Future<Output = Response> + std::marker::Send;
}

pub async fn listen<H: Handler>(
    stream: &mut TcpStream,
    handler: &mut H,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0; 1024];
    let mut msg = String::new();
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let buf = &buf[..n];
        msg += &String::from_utf8_lossy(buf);

        let request = serde_json::from_str(&msg);
        if request.is_ok() {
            msg.clear();
            let request = request?;
            let response = handler.handle(request).await;
            let response = serde_json::to_string(&response)?;
            stream.write_all(response.as_bytes()).await?;
        } else {
            continue;
        }
    }
    Ok(())
}
