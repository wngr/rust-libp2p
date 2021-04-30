use futures::future::BoxFuture;
use libp2p_core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use libp2p_swarm::{NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerUpgrErr};
use std::io;

use crate::{StreamHandle, StreamId};

pub struct StreamingProtocolsHandler;

pub struct StreamingInEvent;

pub enum StreamingProtocolsHandlerEvent {
    NewIncoming { id: StreamId, stream: StreamHandle },
    StreamOpened { id: StreamId },
    OutboundTimeout(StreamId),
    OutboundUnsupportedProtocols(StreamId),
    //InboundTimeout(StreamId),
    //InboundUnsupportedProtocols(StreamId)
}

pub struct StreamingProtocol;
impl UpgradeInfo for StreamingProtocol {
    type Info = &'static [u8];

    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(b"FIXME USER PROVIDED")
    }
}
impl InboundUpgrade<NegotiatedSubstream> for StreamingProtocol {
    type Output = StreamHandle;

    type Error = io::Error;

    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: NegotiatedSubstream, info: Self::Info) -> Self::Future {
        todo!()
    }
}
impl OutboundUpgrade<NegotiatedSubstream> for StreamingProtocol {
    type Output = StreamHandle;

    type Error = io::Error;

    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: NegotiatedSubstream, info: Self::Info) -> Self::Future {
        todo!()
    }
}

impl ProtocolsHandler for StreamingProtocolsHandler {
    type InEvent = StreamingInEvent;

    type OutEvent = StreamingProtocolsHandlerEvent;

    type Error = ProtocolsHandlerUpgrErr<std::io::Error>;

    type InboundProtocol = StreamingProtocol;

    type OutboundProtocol = StreamingProtocol;

    type InboundOpenInfo = StreamId;

    type OutboundOpenInfo = StreamId;

    fn listen_protocol(
        &self,
    ) -> libp2p_swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        todo!()
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        handle: StreamHandle,
        info: Self::InboundOpenInfo,
    ) {
        todo!()
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        handle: StreamHandle,
        info: Self::OutboundOpenInfo,
    ) {
        todo!()
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        todo!()
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: ProtocolsHandlerUpgrErr<io::Error>,
    ) {
        todo!()
    }

    fn connection_keep_alive(&self) -> libp2p_swarm::KeepAlive {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p_swarm::ProtocolsHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        todo!()
    }
}
