use anyhow::{bail, Result};
use log::*;
use rosc::{self, OscMessage, OscPacket, OscType};
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};

pub struct Osc {
    sock: UdpSocket,
    buf: [u8; rosc::decoder::MTU],
    osc_port: u16,
}

impl Osc {
    pub fn new(ip: embedded_svc::ipv4::Ipv4Addr, port: u16) -> Self {
        let recv_addr = SocketAddrV4::new(ip, port);
        let sock = UdpSocket::bind(recv_addr).unwrap();
        let buf = [0u8; rosc::decoder::MTU];

        Self {
            sock,
            buf,
            osc_port: port,
        }
    }

    pub fn run(&mut self, addr: SocketAddr, topic: &str, args: OscType) -> Result<()> {
        let msg_buf = rosc::encoder::encode(&OscPacket::Message(OscMessage {
            addr: topic.to_string(),
            args: vec![args],
        }))?;
        self.sock.send_to(&msg_buf, addr)?;
        Ok(())
    }

    pub fn ping(&mut self) -> Result<()> {
        match self.sock.recv_from(&mut self.buf) {
            Ok((size, addr)) => {
                info!("Received packet with size {size} from: {addr}");
                let (_, packet) = rosc::decoder::decode_udp(&self.buf[..size]).unwrap();
                match packet {
                    OscPacket::Message(msg) => {
                        // the topic, not the address. e.g. /test
                        info!("OSC address: {}", msg.addr);

                        // arguments can be Int(int), Float(float), etc.
                        info!("OSC arguments: {:?}", msg.args);

                        match msg.addr.as_str() {
                            // reply /pong 1 to sender (port will be changed to OSC_DEST_PORT)
                            "/ping" => {
                                let msg_buf =
                                    rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                        addr: "/pong".to_string(),
                                        args: vec![OscType::Int(1)],
                                    }))?;

                                let mut pong_addr = addr.clone();
                                pong_addr.set_port(self.osc_port);

                                info!("Reply /pong to {pong_addr}");
                                self.sock.send_to(&msg_buf, pong_addr)?;
                            }
                            _ => {}
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        info!("OSC Bundle: {bundle:?}");
                    }
                }
                Ok(())
            }
            Err(e) => {
                bail!("Error receiving from socket: {e}");
            }
        }
    }
}
