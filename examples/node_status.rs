#[macro_use]
extern crate uavcan;
extern crate uavcan_socketcan;

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

#[derive(Debug, UavcanStruct, Default)]
struct NodeStatus {
    uptime_sec: u32,
    health: u2,
    mode: u3,
    sub_mode: u3,
    vendor_specific_status_code: u16,
}

impl Message for NodeStatus {
    const TYPE_ID: u16 = 341;
}



fn main() {

    let start_time = time::SystemTime::now();

    let can_interface = CanInterface::open("vcan0").unwrap();
    let node_config = NodeConfig{id: Some(NodeID::new(32))};
    let node = Arc::new(SimpleNode::new(can_interface, node_config));
    let node_rx = node.clone();

    
    std::thread::spawn(move || {

        loop {

            if let Ok(message) = node_rx.receive_message::<NodeStatus>() {
                println!("Received node status frame: {:?}",  message);
            }
            
            thread::sleep(time::Duration::from_millis(10));
            
        }

    });
    
   
    loop {
        let now = time::SystemTime::now();
        let message = NodeStatus{
            uptime_sec: now.duration_since(start_time).unwrap().as_secs() as u32,
            health: u2::new(0),
            mode: u3::new(0),
            sub_mode: u3::new(0),
            vendor_specific_status_code: 0,
        };

        node.transmit_message(message);
        
        thread::sleep(time::Duration::from_millis(1000));
        
    }
}

