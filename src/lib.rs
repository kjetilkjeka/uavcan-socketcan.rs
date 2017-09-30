extern crate socketcan;
extern crate uavcan;

use std::collections::HashMap;

use uavcan::transfer::TransferFrame;
use uavcan::transfer::TransferFrameID;
use uavcan::transfer::TransferInterface;
use uavcan::transfer::FullTransferID;
use uavcan::transfer::TransmitError;

pub struct CanInterface(socketcan::CANSocket);

impl CanInterface {
    pub fn open(ifname: &str) -> Result<Self, socketcan::CANSocketOpenError> {
        let interface = socketcan::CANSocket::open(ifname)?;
        interface.set_nonblocking(true).unwrap();
        Ok(CanInterface(interface))
    }
}

impl TransferInterface for CanInterface {
    type Frame = CanFrame;

    fn transmit(&self, frame: &Self::Frame) -> Result<(), TransmitError> {
        let CanInterface(ref interface) = *self;
        match interface.write_frame(&(*frame).into()) {
            Ok(()) => Ok(()),
            Err(_) => Err(TransmitError::BufferFull), // fix this error message
        }
    }

    fn receive(&self, identifier: Option<&FullTransferID>) -> Option<Self::Frame> {
        if identifier.is_some() {
            unimplemented!("No support for receive by identifier yet");
        }
        let CanInterface(ref interface) = *self;
        match interface.read_frame().ok() {
            Some(frame) => Some(frame.into()),
            None => None,
        }
    }

    fn received_completely(&self) -> &[FullTransferID] {
        let CanInterface(ref _interface) = *self;
        unimplemented!()
    }

}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanFrame {
    id: TransferFrameID,
    dlc: usize,
    data: [u8; 8],
}

impl TransferFrame for CanFrame {
    const MAX_DATA_LENGTH: usize = 8;
    
    fn new(id: TransferFrameID) -> CanFrame {
        CanFrame{id: id, dlc: 0, data: [0; 8]}
    }
    
    fn set_data_length(&mut self, length: usize) {
        assert!(length <= 8);
        self.dlc = length;
    }

    fn data(&self) -> &[u8] {
        &self.data[0..self.dlc]
    }

    fn data_as_mut(&mut self) -> &mut[u8] {
        &mut self.data[0..self.dlc]
    }
    
    fn id(&self) -> TransferFrameID {
        self.id 
    }
}

impl From<socketcan::CANFrame> for CanFrame {
    fn from(frame: socketcan::CANFrame) -> CanFrame {
        let mut data = [0u8; 8];
        for (i, element) in frame.data().iter().enumerate() {
            data[i] = *element;
        }
        
        CanFrame{
            id: TransferFrameID::from(frame.id()),
            dlc: frame.data().len(),
            data: data,
        }
    }
}

impl From<CanFrame> for socketcan::CANFrame {
    fn from(frame: CanFrame) -> socketcan::CANFrame {
        socketcan::CANFrame::new(frame.id().into(), frame.data(), false, false).unwrap()
    }
}


pub struct TransferBuffer(Vec<CanFrame>);

impl TransferBuffer{
    pub fn new() -> Self {
        TransferBuffer(Vec::new())
    }

    pub fn push(&mut self, frame: CanFrame) {
        let TransferBuffer(ref mut vec) = *self;
        vec.push(frame);
    }

    pub fn remove(&mut self) -> CanFrame {
        let TransferBuffer(ref mut vec) = *self;
        vec.remove(0)
    }

    pub fn is_empty(&self) -> bool {
        let TransferBuffer(ref vec) = *self;
        vec.is_empty()
    }
}


pub struct ReceiveBuffer {
    map: HashMap<FullTransferID, TransferBuffer>,
}

impl ReceiveBuffer {
    pub fn new() -> Self {
        ReceiveBuffer{map: HashMap::new()}
    }

    pub fn insert(&mut self, key: &FullTransferID, value: CanFrame) {
        self.map.entry(*key).or_insert(TransferBuffer::new());
        self.map.get_mut(key).unwrap().push(value);
    }

    pub fn remove(&mut self, key: &FullTransferID) -> Option<CanFrame> {
        let (can_frame, empty) = { 
            let transfer_buffer = match self.map.get_mut(key) {
                Some(x) => x,
                None => return None,
            };
            (transfer_buffer.remove(), transfer_buffer.is_empty())
        };

        if empty {
            self.map.remove(key);
        }
        
        Some(can_frame)
    }
}

