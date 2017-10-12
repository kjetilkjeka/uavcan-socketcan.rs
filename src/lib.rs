extern crate socketcan;
extern crate uavcan;

use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::Mutex;

use uavcan::transfer::TransferFrame;
use uavcan::transfer::TransferFrameID;
use uavcan::transfer::TransferInterface;
use uavcan::transfer::FullTransferID;
use uavcan::transfer::TransmitError;

pub struct CanInterface {
    interface: Mutex<socketcan::CANSocket>,
    rx_buffer: Mutex<RefCell<ReceiveBuffer>>,
}

impl CanInterface {
    pub fn open(ifname: &str) -> Result<Self, socketcan::CANSocketOpenError> {
        let interface = socketcan::CANSocket::open(ifname)?;
        interface.set_nonblocking(true).unwrap();
        interface.filter_accept_all().unwrap();
        Ok(CanInterface{interface: Mutex::new(interface), rx_buffer: Mutex::new(RefCell::new(ReceiveBuffer::new())) })
    }

    fn update_receive_buffer(&self) {
        let interface = self.interface.lock().unwrap();
        while let Ok(frame) = interface.read_frame() {
            println!("Frame received in driver");
            let data = self.rx_buffer.lock().unwrap();
            let mut buffer = data.borrow_mut();
            buffer.insert(frame.into());
        }
    }
}

impl<'a> TransferInterface<'a> for CanInterface {
    type Frame = CanFrame;
    type IDContainer = Box<[FullTransferID]>;
    
    fn transmit(&self, frame: &Self::Frame) -> Result<(), TransmitError> {
        let interface = self.interface.lock().unwrap();
        match interface.write_frame(&(*frame).into()) {
            Ok(()) => Ok(()),
            Err(_) => Err(TransmitError::BufferFull), // fix this error message
        }
    }

    fn receive(&self, identifier: &FullTransferID) -> Option<Self::Frame> {
        self.update_receive_buffer();
        let data = self.rx_buffer.lock().unwrap();
        let mut buffer = data.borrow_mut();
        buffer.remove(identifier)
    }

    fn completed_receives(&self, identifier: FullTransferID, mask: FullTransferID) -> Self::IDContainer {
        self.update_receive_buffer();
        let data = self.rx_buffer.lock().unwrap();
        let buffer = data.borrow();
        buffer.completed_transfers(identifier, mask).into_boxed_slice()
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

    pub fn is_complete(&self) -> bool {
        let TransferBuffer(ref vec) = *self;
        vec.iter().any(|&x| x.is_end_frame())
    }
}


pub struct ReceiveBuffer {
    map: HashMap<FullTransferID, TransferBuffer>,
}

impl ReceiveBuffer {
    pub fn new() -> Self {
        ReceiveBuffer{map: HashMap::new()}
    }

    pub fn insert(&mut self, frame: CanFrame) {
        self.map.entry(frame.full_id()).or_insert(TransferBuffer::new());
        self.map.get_mut(&frame.full_id()).unwrap().push(frame);
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

    pub fn completed_transfers(&self, identifier: FullTransferID, mask: FullTransferID) -> Vec<FullTransferID> {
        self.map.iter()
            .filter(|&(key, _value)| key.mask(mask) == identifier.mask(mask))
            .filter(|&(_key, value)| value.is_complete())
            .map(|(key, _value)| key.clone())
            .collect::<Vec<FullTransferID>>()            
    }
}

