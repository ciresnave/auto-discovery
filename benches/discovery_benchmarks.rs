use criterion::{criterion_group, criterion_main, Criterion, BenchmarkGroup};
use auto_discovery::{
    config::DiscoveryConfig,
    protocols::{
        ProtocolManagerBuilder,
        mdns::MdnsProtocol,
        upnp::SsdpProtocol,
        DiscoveryProtocol,
    },
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use std::{net::Ipv4Addr, time::Duration, sync::Arc};
use tokio::runtime::Runtime;
use futures::future::join_all;

const BENCH_SAMPLE_SIZE: usize = 10;
const BENCH_MEASUREMENT_TIME: Duration = Duration::from_secs(30);
const CONCURRENT_OPERATIONS: usize = 5;

/// Initialize benchmark group with common settings
fn init_bench_group(c: &mut Criterion, name: &str) -> BenchmarkGroup<'_, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(BENCH_SAMPLE_SIZE);
    group.measurement_time(BENCH_MEASUREMENT_TIME);
    group
}

/// Benchmark individual protocol discovery
fn protocol_discovery_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DiscoveryConfig::default();

    let mut group = init_bench_group(c, "protocol_discovery");

    // mDNS Discovery
    group.bench_function("mdns_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            let mdns = MdnsProtocol::new(&config).await.unwrap();
            mdns.discover_services(
                vec![ServiceType::new("_http._tcp")],
                Some(Duration::from_secs(1))
            ).await.unwrap()
        });
    });

    // SSDP Discovery
    group.bench_function("ssdp_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            let ssdp = SsdpProtocol::new(&config).await.unwrap();
            ssdp.discover_services(
                vec![ServiceType::new("urn:schemas-upnp-org:service:ContentDirectory:1")],
                Some(Duration::from_secs(1))
            ).await.unwrap()
        });
    });

    group.finish();
}

/// Benchmark service registration
fn service_registration_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DiscoveryConfig::default();

    let mut group = init_bench_group(c, "service_registration");

    // mDNS Registration
    group.bench_function("mdns_registration", |b| {
        b.to_async(&rt).iter(|| async {
            let mut mdns = MdnsProtocol::new(&config).await.unwrap();
            let service = ServiceInfo::new_with_protocol(
                "bench_service",
                ServiceType::new("_bench._tcp"),
                Ipv4Addr::LOCALHOST.into(),
                8080,
                Some(vec![("version", "1.0")]),
                ProtocolType::Mdns,
            );
            mdns.register_service(&service).await.unwrap();
            mdns.unregister_service(&service).await.unwrap()
        });
    });

    // SSDP Registration
    group.bench_function("ssdp_registration", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ssdp = SsdpProtocol::new(&config).await.unwrap();
            let service = ServiceInfo::new_with_protocol(
                "bench_service",
                ServiceType::new("urn:test-bench-service"),
                Ipv4Addr::LOCALHOST.into(),
                8080,
                Some(vec![("version", "1.0")]),
                ProtocolType::Upnp,
            );
            ssdp.register_service(&service).await.unwrap();
            ssdp.unregister_service(&service).await.unwrap()
        });
    });

    group.finish();
}

/// Benchmark concurrent operations
fn concurrent_operations_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DiscoveryConfig::default();

    let mut group = init_bench_group(c, "concurrent_operations");

    // Concurrent mDNS Discoveries
    group.bench_function("concurrent_mdns_discoveries", |b| {
        b.to_async(&rt).iter(|| async {
            let mdns = Arc::new(MdnsProtocol::new(&config).await.unwrap());
            let mut handles = Vec::with_capacity(CONCURRENT_OPERATIONS);

            for i in 0..CONCURRENT_OPERATIONS {
                let mdns = mdns.clone();
                handles.push(tokio::spawn(async move {
                    mdns.discover_services(
                        vec![ServiceType::new(&format!("_bench{}._tcp", i))],
                        Some(Duration::from_secs(1))
                    ).await
                }));
            }

            join_all(handles).await
        });
    });

    // Concurrent SSDP Discoveries
    group.bench_function("concurrent_ssdp_discoveries", |b| {
        b.to_async(&rt).iter(|| async {
            let ssdp = Arc::new(SsdpProtocol::new(&config).await.unwrap());
            let mut handles = Vec::with_capacity(CONCURRENT_OPERATIONS);

            for i in 0..CONCURRENT_OPERATIONS {
                let ssdp = ssdp.clone();
                handles.push(tokio::spawn(async move {
                    ssdp.discover_services(
                        vec![ServiceType::new(&format!("urn:test-bench-{}", i))],
                        Some(Duration::from_secs(1))
                    ).await
                }));
            }

            join_all(handles).await
        });
    });

    group.finish();
}

/// Benchmark protocol manager operations
fn protocol_manager_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = DiscoveryConfig::default();

    let mut group = init_bench_group(c, "protocol_manager");

    // Multi-protocol Discovery
    group.bench_function("multi_protocol_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = ProtocolManagerBuilder::new(config.clone())
                .with_mdns(true)
                .with_upnp(true)
                .build()
                .await
                .unwrap();

            let service_types = vec![
                ServiceType::new("_http._tcp"),
                ServiceType::new("urn:schemas-upnp-org:service:ContentDirectory:1"),
            ];
            
            manager.discover_services(service_types, Duration::from_secs(1)).await.unwrap()
        });
    });

    // Protocol Manager Service Registration
    group.bench_function("manager_service_registration", |b| {
        b.to_async(&rt).iter(|| async {
            let mut manager = ProtocolManagerBuilder::new(config.clone())
                .with_mdns(true)
                .with_upnp(true)
                .build()
                .await
                .unwrap();

            let service = ServiceInfo::new_with_protocol(
                "bench_service",
                ServiceType::new("_bench._tcp"),
                Ipv4Addr::LOCALHOST.into(),
                8080,
                Some(vec![("version", "1.0")]),
                ProtocolType::Any,
            );

            manager.register_service(&service).await.unwrap();
            manager.unregister_service(&service).await.unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    protocol_discovery_benchmark,
    service_registration_benchmark,
    concurrent_operations_benchmark,
    protocol_manager_benchmark,
);
criterion_main!(benches);
