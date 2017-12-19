extern crate socketcan;
extern crate uavcan;

use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use uavcan::transfer::TransferFrame;
use uavcan::transfer::TransferFrameID;
use uavcan::transfer::TransferFrameIDFilter;
use uavcan::transfer::TransferInterface;
use uavcan::transfer::TransferSubscriber;
use uavcan::transfer::FullTransferID;
use uavcan::transfer::IOError;

pub struct CanInterface {
    interface: Arc<socketcan::CANSocket>,
    subscribers: Arc<Mutex<Vec<SubscriberHandle>>>,
    receiver_handle: std::thread::JoinHandle<()>,
}

impl CanInterface {
    pub fn open(ifname: &str) -> Result<Self, socketcan::CANSocketOpenError> {
        let interface = Arc::new(socketcan::CANSocket::open(ifname)?);
        interface.filter_accept_all().unwrap();
        
        let subscribers: Arc<Mutex<Vec<SubscriberHandle>>> = Arc::new(Mutex::new(Vec::new()));

        let interface_thread = interface.clone();
        let subscribers_thread = subscribers.clone();
        
        let receiver_handle = std::thread::spawn(move || {
            loop {
                if let Ok(can_frame) = interface_thread.read_frame() {
                    let can_frame = CanFrame::from(can_frame);
                    for sub in subscribers_thread.lock().unwrap().iter() {
                        if sub.filter.is_match(can_frame.id()) {
                            let mut buffer = sub.buffer.lock().unwrap();
                            buffer.push(can_frame);
                        }
                    }
                }
            }
        });
        
        Ok(CanInterface{
            interface: interface,
            subscribers: subscribers,
            receiver_handle: receiver_handle,
        })
    }
}

impl TransferInterface for CanInterface {
    type Frame = CanFrame;
    type Subscriber = Subscriber;
    
    fn transmit(&self, frame: &Self::Frame) -> Result<(), IOError> {
        match self.interface.write_frame(&(*frame).into()) {
            Ok(()) => Ok(()),
            Err(_) => Err(IOError::BufferExhausted), // fix this error message
        }
    }

    fn subscribe(&self, filter: TransferFrameIDFilter) -> Result<Self::Subscriber, ()> {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let mut subscribers = self.subscribers.lock().unwrap();

        let subscriber_handle = SubscriberHandle {
            filter: filter,
            buffer: buffer.clone(),
        };

        subscribers.push(subscriber_handle);

        Ok(Subscriber{
            buffer: buffer,
        })
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
            id: TransferFrameID::new(frame.id()),
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

pub struct SubscriberHandle {
    buffer: Arc<Mutex<Vec<CanFrame>>>,
    filter: TransferFrameIDFilter,
}

pub struct Subscriber {
    buffer: Arc<Mutex<Vec<CanFrame>>>,
}

impl TransferSubscriber for Subscriber {
    type Frame = CanFrame;

    fn receive(&self, identifier: &TransferFrameID) -> Option<Self::Frame> {
        let mut buffer = self.buffer.lock().unwrap();
        let pos = buffer.iter().position(|x| x.id() == *identifier)?;
        Some(buffer.remove(pos))
    }

    fn retain<F>(&self, f: F) where F: FnMut(&Self::Frame) -> bool {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.retain(f);
    }
    
    fn find<F>(&self, mut predicate: F) -> Option<Self::Frame> where F: FnMut(&Self::Frame) -> bool {
        let buffer = self.buffer.lock().unwrap();
        Some(*buffer.iter().find(|x| predicate(&x))?)
    }
    
}
