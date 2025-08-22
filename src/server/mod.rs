use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use bytes::Bytes;
use tokio::{net::UdpSocket, time::timeout};

use crate::{
    blocklist::{matcher::BlocklistMatcher, service::BlocklistService},
    cache::service::CacheService,
    global::Global,
    middleware::DnsMiddleware,
    resolver::{DnsRequestCtx, DnsResolver},
};

/// DNS Server
pub struct DnsServer<R> {
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    recv_size: usize,
    timeout: Duration,
    middlewares: ArcSwap<Vec<Arc<dyn DnsMiddleware>>>,
    global: Arc<Global>,
}

impl<R: DnsResolver + Send + Sync + 'static> DnsServer<R> {
    pub fn new(bind_addr: SocketAddr, resolver: R, global: Arc<Global>) -> Self {
        Self {
            bind_addr,
            resolver: Arc::new(resolver),
            recv_size: 1232, // edns safe
            timeout: Duration::from_secs(2),
            middlewares: ArcSwap::new(Vec::new().into()),
            global,
        }
    }
    pub fn add_middleware<M>(&self, mw: M)
    where
        M: DnsMiddleware + 'static,
    {
        let cur = self.middlewares.load();
        let mut v = Vec::with_capacity(cur.len() + 1);
        v.extend(cur.iter().cloned());
        v.push(Arc::new(mw));
        self.middlewares.store(Arc::new(v));
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.bind_addr).await?);

        loop {
            let mut query = vec![0u8; self.recv_size];
            let sock = socket.clone();
            let (len, client) = sock.recv_from(&mut query).await?;

            query.truncate(len);

            let resolver = self.resolver.clone();

            let duration = self.timeout;

            let guard = self.middlewares.load();
            let middlewares = guard.clone();
            let global = self.global.clone();

            tokio::spawn(async move {
                let ctx = DnsRequestCtx::new(&query, global);
                if let Ok(Some(resp)) = run_middlewares(middlewares, &ctx).await {
                    let _ = sock.send_to(&resp, client).await;
                    return;
                }

                match timeout(duration, resolver.resolve(&ctx)).await {
                    Ok(Ok(resp)) => {
                        let _ = sock.send_to(&resp, client).await;
                    }
                    Ok(Err(e)) => {
                        println!("error: {}", e);
                    }
                    Err(e) => println!("timeout error: {}", e),
                }
            });
        }
    }
}

pub async fn run_middlewares(
    mws: std::sync::Arc<Vec<Arc<dyn DnsMiddleware>>>,
    ctx: &DnsRequestCtx<'_>,
) -> anyhow::Result<Option<Bytes>> {
    for m in mws.iter() {
        if let Some(resp) = m.on_query(ctx).await? {
            return Ok(Some(resp));
        }
    }
    Ok(None)
}
