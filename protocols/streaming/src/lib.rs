//! TODO:
//!  * How to add arbitrary protocols on top of it, how to encode etc.
//!  * How to mark idle connections. Maybe check refcount of the stream?
//!    Dropped -> Idle? This can work on the outbound side, but what about the inbound side?
//!    How to signal close?
use std::{
    collections::{HashSet, VecDeque},
    sync::{atomic::AtomicU64, Arc},
    task::Poll,
};

use futures::{AsyncRead, AsyncWrite};
use handler::{StreamingProtocolsHandler, StreamingProtocolsHandlerEvent};
use libp2p_core::PeerId;
use libp2p_swarm::{NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction};

mod handler;

#[derive(Default)]
pub struct Streaming {
    // open_streams: BTreeMap<(PeerId, ConnectionId), SmallVec<[StreamId; 2]>>,
    /// Pending events to return from `poll`.
    pending_events: VecDeque<NetworkBehaviourAction<OutboundStreamId, StreamingEvent>>,
    /// The next (inbound) request ID.
    next_inbound_id: Arc<AtomicU64>,
    /// The next (local) request ID.
    next_stream_id: OutboundStreamId,
    connected: HashSet<PeerId>,
}
/// FIXME docs
/// The ID of an inbound or outbound request.
///
/// Note: [`RequestId`]'s uniqueness is only guaranteed between two
/// inbound and likewise between two outbound requests. There is no
/// uniqueness guarantee in a set of both inbound and outbound
/// [`RequestId`]s nor in a set of inbound or outbound requests
/// originating from different [`RequestResponse`] behaviours.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct InboundStreamId(u64);

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct OutboundStreamId(u64);
#[pin_project::pin_project]
#[derive(Debug)]
// TODO: keep track whether connection is idle!?
pub struct StreamHandle {
    #[pin]
    inner: NegotiatedSubstream,
}
impl StreamHandle {
    fn new(inner: NegotiatedSubstream) -> Self {
        Self { inner }
    }
}
impl AsyncRead for StreamHandle {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.inner.poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.inner.poll_read_vectored(cx, bufs)
    }
}

impl AsyncWrite for StreamHandle {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.inner.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.inner.poll_close(cx)
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.inner.poll_write_vectored(cx, bufs)
    }
}
// Exposed swarm events
#[derive(Debug)]
pub enum StreamingEvent {
    // New connection from remote
    NewIncoming {
        peer_id: PeerId,
        id: InboundStreamId,
        stream: StreamHandle,
    },
    StreamOpened {
        peer_id: PeerId,
        id: OutboundStreamId,
        stream: StreamHandle,
    },
    OutboundFailure {
        peer_id: PeerId,
        id: OutboundStreamId,
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
#[derive(thiserror::Error, Debug)]
pub enum StreamingError {
    #[error("Peer {0} is not connected")]
    NotConnected(PeerId),
}
type Result<T> = std::result::Result<T, StreamingError>;
impl Streaming {
    pub fn open_stream(&mut self, peer_id: PeerId) -> Result<OutboundStreamId> {
        if !self.connected.contains(&peer_id) {
            log::warn!("Rejecting opening stream for disconnected peer {}", peer_id);
            return Err(StreamingError::NotConnected(peer_id));
        }
        log::trace!("Opening stream for {}", peer_id);
        let stream_id = self.next_stream_id();
        // TODO: dial peer
        let action = NetworkBehaviourAction::NotifyHandler {
            peer_id,
            // TODO one with specific connection
            handler: libp2p_swarm::NotifyHandler::Any,
            event: stream_id,
        };
        self.pending_events.push_back(action);
        Ok(stream_id)
    }
    fn next_stream_id(&mut self) -> OutboundStreamId {
        let stream_id = self.next_stream_id;
        self.next_stream_id.0 += 1;
        log::trace!("Handing out stream_id {:?}", stream_id);
        stream_id
    }
}
impl NetworkBehaviour for Streaming {
    type ProtocolsHandler = StreamingProtocolsHandler;

    type OutEvent = StreamingEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        log::trace!("new_handler");
        StreamingProtocolsHandler::new(self.next_inbound_id.clone())
    }

    fn addresses_of_peer(&mut self, _: &libp2p_core::PeerId) -> Vec<libp2p_core::Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, peer_id: &libp2p_core::PeerId) {
        log::trace!("{} connected", peer_id);
        self.connected.insert(*peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &libp2p_core::PeerId) {
        log::trace!("{} disconnected", peer_id);
        self.connected.remove(peer_id);
    }

    fn inject_event(
        &mut self,
        peer_id: libp2p_core::PeerId,
        _: libp2p_core::connection::ConnectionId,
        event: StreamingProtocolsHandlerEvent,
    ) {
        log::trace!("inject_event from {}: {:?} ", peer_id, event);
        match event {
            StreamingProtocolsHandlerEvent::NewIncoming { id, stream } => {
                let ev = StreamingEvent::NewIncoming {
                    peer_id,
                    id,
                    stream,
                };
                self.pending_events
                    .push_back(NetworkBehaviourAction::GenerateEvent(ev));
            }
            StreamingProtocolsHandlerEvent::StreamOpened { id, stream } => {
                let ev = StreamingEvent::StreamOpened {
                    id,
                    peer_id,
                    stream,
                };
                self.pending_events
                    .push_back(NetworkBehaviourAction::GenerateEvent(ev));
            }
            StreamingProtocolsHandlerEvent::OutboundTimeout(_) => {
                todo!()
            }
            StreamingProtocolsHandlerEvent::OutboundUnsupportedProtocols(_) => {
                todo!()
            }
        }
    }

    fn poll(
        &mut self,
        _: &mut std::task::Context<'_>,
        _: &mut impl libp2p_swarm::PollParameters,
    ) -> std::task::Poll<libp2p_swarm::NetworkBehaviourAction<OutboundStreamId, Self::OutEvent>>
    {
        if let Some(ev) = self.pending_events.pop_front() {
            log::trace!("poll {:?}", ev);
            return Poll::Ready(ev);
        } else if self.pending_events.capacity() > EMPTY_QUEUE_SHRINK_THRESHOLD {
            self.pending_events.shrink_to_fit();
        }

        Poll::Pending
    }
}

/// Internal threshold for when to shrink the capacity
/// of empty queues. If the capacity of an empty queue
/// exceeds this threshold, the associated memory is
/// released.
const EMPTY_QUEUE_SHRINK_THRESHOLD: usize = 100;
