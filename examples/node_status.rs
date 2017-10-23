#[macro_use]
extern crate uavcan;
extern crate uavcan_socketcan;

use std::{thread, time};
use std::sync::Arc;

use uavcan::types::*;
use uavcan::{
    Frame,
    Message,
    NodeID,
};

use uavcan::transfer::TransferInterface;
use uavcan::transfer::FullTransferID;
use uavcan::transfer::TransferID;

use uavcan::frame_disassembler::FrameDisassembler;
use uavcan::frame_assembler::{
    FrameAssembler,
    AssemblerResult,
};

use uavcan_socketcan::CanFrame;
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

    let can_interface = Arc::new(CanInterface::open("vcan0").unwrap());
    let can_interface_rx = can_interface.clone();
    
    std::thread::spawn(move || {

        let identifier = FullTransferID {
            frame_id: NodeStatus::id(0, NodeID::new(0)),
            transfer_id: TransferID::new(0),
        };
        
        let mask = identifier.clone();
        
        loop {
            if let Some(id) = can_interface_rx.completed_receive(identifier, mask) {
                let mut assembler = FrameAssembler::new();
                loop {
                    match assembler.add_transfer_frame(can_interface_rx.receive(&id).unwrap()) {
                        Ok(AssemblerResult::Ok) => (),
                        Ok(AssemblerResult::Finished) => break,
                        Err(_) => break,
                    }
                }

                let node_status_frame: Frame<NodeStatus> = assembler.build().unwrap();
                println!("Received node status frame: {:?}",  node_status_frame);
            }
                 
            thread::sleep(time::Duration::from_millis(10));
            
        }

    });

   
    loop {
        let now = time::SystemTime::now();
        let uavcan_frame = Frame::from_message(
            NodeStatus{
                uptime_sec: now.duration_since(start_time).unwrap().as_secs() as u32,
                health: u2::new(0),
                mode: u3::new(0),
                sub_mode: u3::new(0),
                vendor_specific_status_code: 0,
            }, 0, NodeID::new(32));

        let mut generator = FrameDisassembler::from_uavcan_frame(uavcan_frame, TransferID::new(0));
        let can_frame = generator.next_transfer_frame::<CanFrame>().unwrap();
                
        can_interface.transmit(&can_frame).unwrap();

        thread::sleep(time::Duration::from_millis(1000));
        
    }
}

