use std::{time::Instant, collections::{BTreeMap, BinaryHeap, BTreeSet}};

use async_channel::Receiver;
use bytes::BytesMut;

use crate::{packets::{publish::Publish, packets::Packet, QoS, puback::{PubAck, PubAckProperties}, reason_codes::PubAckReasonCode, subscribe::Subscribe, unsubscribe::Unsubscribe}, error::MqttError, available_packet_ids::AvailablePacketIds};

#[derive(Debug, Clone)]
pub struct State {
    /// Status of last ping
    pub await_ping_resp: bool,

    pub(crate) apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    pub(crate) outgoing_sub: BTreeMap<u16, Subscribe>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    pub(crate) outgoing_unsub: BTreeMap<u16, Unsubscribe>,
    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    pub(crate) outgoing_pub: BTreeMap<u16, Publish>,
    /// Packet ids of released QoS 2 publishes
    pub(crate) outgoing_rel: BTreeSet<u16>,

    /// Packets on incoming QoS 2 publishes
    pub(crate) incoming_pub: BTreeSet<u16>,
}

impl State{
    pub fn new(receive_maximum: u16) -> (Self, Receiver<u16>){

        let (apkid, r) = AvailablePacketIds::new(receive_maximum);

        let state = Self{
            await_ping_resp: false,
            // last_incoming: Instant::now(),
            // last_outgoing: Instant::now(),

            apkid,

            // inflight: 0,
            // max_inflight: receive_maximum,
            outgoing_sub: BTreeMap::new(),
            outgoing_unsub: BTreeMap::new(),
            outgoing_pub: BTreeMap::new(),
            outgoing_rel: BTreeSet::new(),
            incoming_pub: BTreeSet::new(),
            // write: BytesMut::with_capacity(1024 * 100),
            // manual_acks: todo!(),
        };

        (state, r)
    }

    

}