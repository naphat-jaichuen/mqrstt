#[cfg(feature = "tokio")]
#[cfg(test)]
mod tokio_e2e {

    use futures_concurrency::future::Join;

    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    use crate::{
        client::AsyncClient,
        connect_options::ConnectOptions,
        create_tokio_tcp,
        error::ClientError,
        event_handler::EventHandler,
        packets::{
            packets::{Packet, PacketType},
            QoS,
        },
    };

    use crate::tests::stages::qos_2::TestPubQoS2;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pub_qos_2() {
        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let opt = ConnectOptions::new("broker.emqx.io".to_string(), 1883, "test123123".to_string(), None);
        // let opt = ConnectOptions::new("127.0.0.1".to_string(), 1883, "test123123".to_string(), None);

        let (mut mqtt_network, handler, client) = create_tokio_tcp(opt);

        let network = tokio::task::spawn(async move { dbg!(mqtt_network.run().await) });

        let client_cloned = client.clone();
        let event_handler = tokio::task::spawn(async move {
            let mut custom_handler = TestPubQoS2::new(client_cloned);
            dbg!(handler.handle(&mut custom_handler).await)
        });

        let sender = tokio::task::spawn(async move {
            client
                .publish(QoS::ExactlyOnce, false, "test".to_string(), "123456789")
                .await?;

            let lol = smol::future::pending::<Result<(), ClientError>>();
            lol.await
        });

        dbg!((network, event_handler, sender).join().await);
    }
}

#[cfg(feature = "smol")]
#[cfg(test)]
mod smol_e2e {

    use futures_concurrency::future::Join;

    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    use crate::{
        client::AsyncClient,
        connect_options::ConnectOptions,
        create_smol_tls,
        error::ClientError,
        event_handler::EventHandler,
        packets::{
            packets::{Packet, PacketType},
            QoS,
        }, connections::transport::TlsConfig, tests::resources::EMQX,
    };

    use crate::tests::stages::qos_2::TestPubQoS2;

    #[test]
    fn test_pub_tcp_qos_2() {
        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let config = TlsConfig::Simple{
            ca: EMQX.to_vec(),
            alpn: None,
            client_auth: None,
        };

        let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string(), Some(config));

        let (mut mqtt_network, handler, client) = create_smol_tls(opt);

        smol::block_on(mqtt_network.run()).unwrap()
    }
}
