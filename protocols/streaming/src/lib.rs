use handler::{StreamingInEvent, StreamingProtocolsHandler, StreamingProtocolsHandlerEvent};
use libp2p_core::PeerId;
use libp2p_swarm::NetworkBehaviour;

mod handler;

pub struct Streaming;
// Who's responsible on creating that?
pub struct StreamId;

pub enum InboundMessage {}

pub struct StreamHandle;
//
// Exposed swarm events
pub enum StreamingEvent {
    // New connection from remote
    NewIncoming {
        peer: PeerId,
        id: StreamId,
        stream: StreamHandle,
    },
    StreamOpened {
        peer: PeerId,
        id: StreamId,
    },
    OutboundFailure {
        peer: PeerId,
        id: StreamId,
        error: OutboundFailure,
    },
    //InboundFailure{peer: PeerId, id: StreamId, }, // error
    // OutboundTimeout(StreamId),
    // OutboundUnsupportedProtocols(StreamId),
}
/// Possible failures occurring in the context of sending
/// an outbound request and receiving the response.
#[derive(Debug, Clone, PartialEq)]
pub enum OutboundFailure {
    /// The request could not be sent because a dialing attempt failed.
    DialFailure,
    /// The request timed out before a response was received.
    ///
    /// It is not known whether the request may have been
    /// received (and processed) by the remote peer.
    Timeout,
    /// The connection closed before a response was received.
    ///
    /// It is not known whether the request may have been
    /// received (and processed) by the remote peer.
    ConnectionClosed,
    /// The remote supports none of the requested protocols.
    UnsupportedProtocols,
}

impl NetworkBehaviour for Streaming {
    type ProtocolsHandler = StreamingProtocolsHandler;

    type OutEvent = StreamingEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        todo!()
    }

    fn addresses_of_peer(&mut self, peer_id: &libp2p_core::PeerId) -> Vec<libp2p_core::Multiaddr> {
        todo!()
    }

    fn inject_connected(&mut self, peer_id: &libp2p_core::PeerId) {
        todo!()
    }

    fn inject_disconnected(&mut self, peer_id: &libp2p_core::PeerId) {
        todo!()
    }

    fn inject_event(
        &mut self,
        peer_id: libp2p_core::PeerId,
        connection: libp2p_core::connection::ConnectionId,
        event: StreamingProtocolsHandlerEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        params: &mut impl libp2p_swarm::PollParameters,
    ) -> std::task::Poll<libp2p_swarm::NetworkBehaviourAction<StreamingInEvent, Self::OutEvent>>
    {
        todo!()
    }
}
