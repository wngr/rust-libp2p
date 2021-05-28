use async_std::prelude::*;
use futures::{channel::mpsc, SinkExt, StreamExt};
use libp2p_core::{
    identity::{self},
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, PeerId,
};
use libp2p_noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p_streaming::Streaming;
use libp2p_swarm::{Swarm, SwarmEvent};
use libp2p_tcp::TcpConfig;

#[async_std::test]
async fn smoke() -> anyhow::Result<()> {
    env_logger::init();

    let (peer1_id, trans) = mk_transport();

    let mut swarm1 = Swarm::new(trans, Streaming::default(), peer1_id);

    let (peer2_id, trans) = mk_transport();
    let mut swarm2 = Swarm::new(trans, Streaming::default(), peer2_id);

    let addr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    swarm1.listen_on(addr).unwrap();

    let (mut tx_addr, mut rx_addr) = mpsc::channel(1);
    async_std::task::spawn(async move {
        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    tx_addr.send(addr).await.unwrap();
                }
                SwarmEvent::ListenerError { error } => panic!("{}", error),
                SwarmEvent::Behaviour(libp2p_streaming::StreamingEvent::NewIncoming {
                    peer_id,
                    mut stream,
                    ..
                }) => {
                    assert_eq!(peer_id, peer2_id);
                    stream.write_all(b"Hello").await.unwrap();
                    stream.flush().await.unwrap();

                    let mut out = vec![0; 32];
                    let n = stream.read(&mut out).await.unwrap();
                    out.truncate(n);
                    assert_eq!(out, b"World!");
                    break;
                }
                x => {
                    log::info!("swarm1: {:?}", x);
                }
            };
        }
    });

    let addr = rx_addr.next().await.unwrap();

    // swarm2.behaviour_mut().add_address(&peer1_id, addr.clone());
    swarm2.dial_addr(addr.clone())?;

    let mut stream_id = None;
    loop {
        match swarm2.next_event().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                assert_eq!(peer1_id, peer_id);

                stream_id.replace(swarm2.behaviour_mut().open_stream(peer1_id).unwrap());
            }
            SwarmEvent::Behaviour(libp2p_streaming::StreamingEvent::StreamOpened {
                id,
                peer_id,
                mut stream,
            }) => {
                assert_eq!(peer1_id, peer_id);
                assert_eq!(Some(id), stream_id);
                let mut out = vec![0; 32];
                let n = stream.read(&mut out).await.unwrap();
                out.truncate(n);
                assert_eq!(out, b"Hello");

                stream.write_all(b"World!").await.unwrap();
                stream.flush().await.unwrap();
                break;
            }
            x => {
                log::info!("swarm2: {:?}", x);
            }
        }
    }

    Ok(())
}

fn mk_transport() -> (PeerId, transport::Boxed<(PeerId, StreamMuxerBox)>) {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = id_keys.public().into_peer_id();
    let noise_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&id_keys)
        .unwrap();
    (
        peer_id,
        TcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(libp2p_yamux::YamuxConfig::default())
            .boxed(),
    )
}
