use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use actix::prelude::*;

use serde;
use serde_json::{self, json};

/// Chat server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendMessageRequest(pub String);

/// Message for chat server communications

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<SendMessageRequest>,
    pub cliend_id: String,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastRequest {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
}

#[derive(Debug)]
struct Client {
    cliend_id: String,
    addr: Recipient<SendMessageRequest>,
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat session.
///
/// Implementation is very naïve.
#[derive(Debug)]
pub struct EventServer {
    sessions: HashMap<usize, Client>,
    next_id: usize,
}

impl EventServer {
    pub fn new() -> EventServer {
        EventServer {
            sessions: HashMap::new(),
            next_id: 0,
        }
    }
}

impl EventServer {
    /// Send message to all users in the room
    fn broadcast(&self, message: &str, skip_id: usize) {
        println!("播报事件: {}", message);

        for (id, client) in &self.sessions {
            if *id == skip_id {
                continue;
            }
            let _ = client.addr.do_send(SendMessageRequest(message.to_owned()));
        }
    }
}

/// Make actor from `ChatServer`
impl Actor for EventServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// 处理客户端连接事件
impl Handler<Connect> for EventServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // println!("Someone joined");

        // 分配数字ID
        self.next_id += 1;
        let id = self.next_id;

        // 保存
        self.sessions.insert(
            id,
            Client {
                cliend_id: msg.cliend_id.clone(),
                addr: msg.addr,
            },
        );

        let e = json!({
            "type": "connect",
            "client_id": msg.cliend_id,
            "time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        });

        let e = serde_json::to_string(&e);
        let e = e.unwrap();
        let e = e.as_ref();

        // 播报
        self.broadcast(e, id);

        id
    }
}

/// 客户端断开连接
impl Handler<Disconnect> for EventServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        // println!("Someone disconnected");

        // 移除客户端
        let session = self.sessions.remove(&msg.id);

        if let Some(session) = session {
            // 播报
            let e = json!({
                "type": "disconnect",
                "client_id": session.cliend_id,
                "time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
            });
            let e = serde_json::to_string(&e);
            let e = e.unwrap();
            let e = e.as_ref();

            // 播报
            self.broadcast(e, msg.id);
        }
    }
}

#[derive(serde::Deserialize, Debug)]
struct Json;

/// 处理客户端的播报请求
impl Handler<BroadcastRequest> for EventServer {
    type Result = ();
    fn handle(&mut self, msg: BroadcastRequest, _: &mut Context<Self>) {
        // 获取client_id
        let session = self.sessions.get(&msg.id);

        if session.is_none() {
            println!("client不存在");
            return;
        }

        let session = session.unwrap();

        // json解析

        let res: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(&msg.msg);

        match res {
            Ok(mut json) => {
                let o = json.as_object_mut();

                if let Some(o) = o {
                    o.insert(
                        "client_id".to_owned(),
                        serde_json::Value::String(session.cliend_id.clone()),
                    );

                    // 发送给所有客户端

                    let res = serde_json::to_string(&json);

                    match res {
                        Ok(res) => {
                            self.broadcast(res.as_str(), msg.id);
                        }

                        Err(e) => {
                            println!("json序列化失败: {}", e);
                        }
                    }
                }
            }

            Err(e) => {
                println!("json解析失败: {}", e);
            }
        }
    }
}
