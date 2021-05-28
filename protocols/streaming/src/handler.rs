use futures::future;
use libp2p_core::{
    upgrade::NegotiationError, InboundUpgrade, OutboundUpgrade, UpgradeError, UpgradeInfo,
};
use libp2p_swarm::{
    KeepAlive, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent,
    ProtocolsHandlerUpgrErr, SubstreamProtocol,
};
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};

use crate::{InboundStreamId, OutboundStreamId, StreamHandle, EMPTY_QUEUE_SHRINK_THRESHOLD};

pub struct StreamingProtocolsHandler {
    /// The keep-alive timeout of idle connections. A connection is considered
    /// idle if there are no outbound substreams.
    // keep_alive_timeout: Duration,
    /// The timeout for inbound and outbound substreams (i.e. request
    /// and response processing).
    substream_timeout: Duration,
    /// The current connection keep-alive.
    keep_alive: KeepAlive,
    /// Queue of events to emit in `poll()`.
    pending_events: VecDeque<StreamingProtocolsHandlerEvent>,
    /// Outbound upgrades waiting to be emitted as an `OutboundSubstreamRequest`.
    outbound: VecDeque<OutboundStreamId>,
    //    /// Inbound upgrades waiting for the incoming request.
    //    inbound: FuturesUnordered<BoxFuture<'static,
    //        Result<
    //            ((RequestId, TCodec::Request), oneshot::Sender<TCodec::Response>),
    //            oneshot::Canceled
    //        >>>,
    // open_streams: SmallVec<[StreamId; 2]>,
    inbound_stream_id: Arc<AtomicU64>,

    /// A pending fatal error that results in the connection being closed.
    pending_error: Option<ProtocolsHandlerUpgrErr<Infallible>>,
}

impl StreamingProtocolsHandler {
    pub(crate) fn new(inbound_stream_id: Arc<AtomicU64>) -> Self {
        Self {
            // keep_alive_timeout: Duration::from_millis(10_000),
            // TODO: needed?
            substream_timeout: Duration::from_millis(500),
            keep_alive: KeepAlive::Yes,
            pending_events: VecDeque::default(),
            outbound: VecDeque::default(),
            // open_streams: SmallVec::new(),
            inbound_stream_id,
            pending_error: None,
        }
    }
}

#[derive(Debug)]
pub enum StreamingProtocolsHandlerEvent {
    NewIncoming {
        id: InboundStreamId,
        stream: StreamHandle,
    },
    StreamOpened {
        id: OutboundStreamId,
        stream: StreamHandle,
    },
    OutboundTimeout(OutboundStreamId),
    OutboundUnsupportedProtocols(OutboundStreamId),
    //InboundTimeout(StreamId),
    //InboundUnsupportedProtocols(StreamId)
}

#[derive(Debug)]
pub struct StreamingProtocol;
impl UpgradeInfo for StreamingProtocol {
    type Info = &'static [u8];

    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        // must start with '/'
        std::iter::once(b"/streaming/raw/1.0.0")
    }
}

// Incoming request
impl InboundUpgrade<NegotiatedSubstream> for StreamingProtocol {
    type Output = StreamHandle;

    type Error = std::convert::Infallible;

    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: NegotiatedSubstream, _: Self::Info) -> Self::Future {
        future::ok(StreamHandle::new(socket))
    }
}

// Outbound request
impl OutboundUpgrade<NegotiatedSubstream> for StreamingProtocol {
    type Output = StreamHandle;

    type Error = std::convert::Infallible;

    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: NegotiatedSubstream, _: Self::Info) -> Self::Future {
        future::ok(StreamHandle::new(socket))
    }
}

// One handler per connection. One connection can have multiple substreams.
impl ProtocolsHandler for StreamingProtocolsHandler {
    type InEvent = OutboundStreamId;

    type OutEvent = StreamingProtocolsHandlerEvent;

    type Error = ProtocolsHandlerUpgrErr<Infallible>;

    type InboundProtocol = StreamingProtocol;

    type OutboundProtocol = StreamingProtocol;

    type InboundOpenInfo = InboundStreamId;

    type OutboundOpenInfo = OutboundStreamId;

    fn listen_protocol(
        &self,
    ) -> libp2p_swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        let stream_id = InboundStreamId(self.inbound_stream_id.fetch_add(1, Ordering::Relaxed));
        log::trace!("new listen_protocol with stream_id {:?}", stream_id);
        SubstreamProtocol::new(StreamingProtocol, stream_id)
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        handle: StreamHandle,
        info: Self::InboundOpenInfo,
    ) {
        log::trace!("New Inbound stream {:?}", info);
        let ev = StreamingProtocolsHandlerEvent::NewIncoming {
            id: info,
            stream: handle,
        };
        self.pending_events.push_back(ev);
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        handle: StreamHandle,
        info: Self::OutboundOpenInfo,
    ) {
        log::trace!("New outbound stream {:?}", info);
        let ev = StreamingProtocolsHandlerEvent::StreamOpened {
            id: info,
            stream: handle,
        };
        self.pending_events.push_back(ev);
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        log::trace!("inject_event {:?}", event);
        self.keep_alive = KeepAlive::Yes;
        self.outbound.push_back(event);
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        err: ProtocolsHandlerUpgrErr<std::convert::Infallible>,
    ) {
        match err {
            ProtocolsHandlerUpgrErr::Timeout => {
                self.pending_events
                    .push_back(StreamingProtocolsHandlerEvent::OutboundTimeout(info));
            }
            ProtocolsHandlerUpgrErr::Upgrade(UpgradeError::Select(NegotiationError::Failed)) => {
                self.pending_events
                    .push_back(StreamingProtocolsHandlerEvent::OutboundUnsupportedProtocols(info))
            }
            _ => {
                // Anything else is considered a fatal error or misbehaviour of
                // the remote peer and results in closing the connection.
                self.pending_error = Some(err);
            }
        }
    }

    fn connection_keep_alive(&self) -> libp2p_swarm::KeepAlive {
        self.keep_alive
    }

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p_swarm::ProtocolsHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        log::trace!("Behaviour: poll");
        // Check for (fatal) error

        if let Some(err) = self.pending_error.take() {
            // The handler will not be polled again by the `Swarm`.
            return Poll::Ready(ProtocolsHandlerEvent::Close(err));
        }
        // Drain pending events.
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(ProtocolsHandlerEvent::Custom(event));
        } else if self.pending_events.capacity() > EMPTY_QUEUE_SHRINK_THRESHOLD {
            self.pending_events.shrink_to_fit();
        }
        // Emit outbound requests.
        if let Some(info) = self.outbound.pop_front() {
            return Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(StreamingProtocol, info)
                    .with_timeout(self.substream_timeout),
            });
        }

        debug_assert!(self.outbound.is_empty());

        if self.outbound.capacity() > EMPTY_QUEUE_SHRINK_THRESHOLD {
            self.outbound.shrink_to_fit();
        }

        // if self.inbound.is_empty() && self.keep_alive.is_yes() {
        //     // No new inbound or outbound requests. However, we may just have
        //     // started the latest inbound or outbound upgrade(s), so make sure
        //     // the keep-alive timeout is preceded by the substream timeout.
        //     let until = Instant::now() + self.substream_timeout + self.keep_alive_timeout;
        //     self.keep_alive = KeepAlive::Until(until);
        // }

        Poll::Pending
    }
}
