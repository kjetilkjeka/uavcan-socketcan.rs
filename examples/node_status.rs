#[macro_use]
extern crate uavcan;
extern crate bit_field;
extern crate uavcan_socketcan;

use std::{thread, time};
use std::sync::Arc;

use uavcan::types::*;
use uavcan::{
    PrimitiveType,
    Frame,
    MessageFrameHeader,
    Header,
};

use uavcan::transfer::TransferInterface;
use uavcan::transfer::FullTransferID;
use uavcan::transfer::TransferID;

use uavcan::frame_disassembler::FrameDisassembler;
use uavcan::frame_assembler::{
    FrameAssembler,
    AssemblerResult,
};

use bit_field::BitField;

use uavcan_socketcan::CanFrame;
use uavcan_socketcan::CanInterface;

#[derive(Debug, UavcanStruct, Default)]
struct NodeStatus {
    uptime_sec: Uint32,
    health: Uint2,
    mode: Uint3,
    sub_mode: Uint3,
    vendor_specific_status_code: Uint16,
}
message_frame_header!(NodeStatusHeader, 341);
uavcan_frame!(derive(Debug), NodeStatusFrame, NodeStatusHeader, NodeStatus, 0);

fn main() {

    let start_time = time::SystemTime::now();

    let can_interface = Arc::new(CanInterface::open("vcan0").unwrap());
    let can_interface_rx = can_interface.clone();
    
    std::thread::spawn(move || {

        let identifier = FullTransferID {
            frame_id: NodeStatusHeader::new(0, 0).id(),
            transfer_id: TransferID::from(0),
        };
        
        let mask = identifier.clone();
        
        loop {
            if let Some(id) = can_interface_rx.completed_receives(identifier, mask).first() {
                let mut assembler = FrameAssembler::new();
                loop {
                    match assembler.add_transfer_frame(can_interface_rx.receive(&id).unwrap()) {
                        Ok(AssemblerResult::Ok) => (),
                        Ok(AssemblerResult::Finished) => break,
                        Err(_) => break,
                    }
                }

                let node_status_frame: NodeStatusFrame = assembler.build().unwrap();
                println!("Received node status frame: {:?}",  node_status_frame);
            }
                 
            thread::sleep(time::Duration::from_millis(10));
            
        }

    });

   
    loop {
        let now = time::SystemTime::now();
        let uavcan_frame = NodeStatusFrame::from_parts(
            NodeStatusHeader::new(0, 32),
            NodeStatus{
                uptime_sec: (now.duration_since(start_time).unwrap().as_secs() as u32).into(),
                health: 0.into(),
                mode: 0.into(),
                sub_mode: 0.into(),
                vendor_specific_status_code: 0.into(),
            }
        );

        let mut generator = FrameDisassembler::from_uavcan_frame(uavcan_frame, 0.into());
        let can_frame = generator.next_transfer_frame::<CanFrame>().unwrap();
                
        can_interface.transmit(&can_frame).unwrap();

        thread::sleep(time::Duration::from_millis(1000));
        
    }
}

