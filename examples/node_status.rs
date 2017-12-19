#[macro_use]
extern crate uavcan;
extern crate uavcan_socketcan;
extern crate dsdl;


use std::{thread, time};
use std::sync::Arc;

use uavcan::types::*;
use uavcan::{
    Message,
    NodeID,
    NodeConfig,
    Node,
    SimpleNode,
};

use uavcan_socketcan::CanInterface;

fn main() {

    let start_time = time::SystemTime::now();

    let can_interface = CanInterface::open("vcan0").unwrap();
    let node_config = NodeConfig{id: Some(NodeID::new(32))};
    let node = Arc::new(SimpleNode::new(&can_interface, node_config));
    let subscriber = node.subscribe::<dsdl::uavcan::protocol::NodeStatus>().unwrap();

    
    std::thread::spawn(move || {

        loop {
            if let Some(receive_res) = subscriber.receive() {
                let message = receive_res.unwrap();
                println!("Received node status frame: {:?}",  message);
            }
            
            thread::sleep(time::Duration::from_millis(10));
            
        }

    });
    
   
    loop {
        let now = time::SystemTime::now();
        let message = dsdl::uavcan::protocol::NodeStatus{
            uptime_sec: now.duration_since(start_time).unwrap().as_secs() as u32,
            health: u2::new(0),
            mode: u3::new(0),
            sub_mode: u3::new(0),
            vendor_specific_status_code: 0,
        };

        node.broadcast(message);
        
        thread::sleep(time::Duration::from_millis(1000));
        
    }
}

