#[macro_use]
extern crate uavcan;
extern crate bit_field;
extern crate uavcan_socketcan;

use std::{thread, time};

use uavcan::types::*;
use uavcan::{
    PrimitiveType,
    Frame,
    MessageFrameHeader,
};

use uavcan::transfer::TransferInterface;

use uavcan::frame_disassembler::FrameDisassembler;

use bit_field::BitField;

use uavcan_socketcan::CanFrame;
use uavcan_socketcan::CanInterface;

#[derive(UavcanStruct, Default)]
struct NodeStatus {
    uptime_sec: Uint32,
    health: Uint2,
    mode: Uint3,
    sub_mode: Uint3,
    vendor_specific_status_code: Uint16,
}
message_frame_header!(NodeStatusHeader, 341);
uavcan_frame!(NodeStatusFrame, NodeStatusHeader, NodeStatus, 0);

fn main() {

    let start_time = time::SystemTime::now();
    let can_interface = CanInterface::open("vcan0").unwrap();
    
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
