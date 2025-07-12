use criterion::{criterion_group, criterion_main, Criterion};
use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use std::time::Duration;
use tokio::runtime::Runtime;

const BENCH_MEASUREMENT_TIME: Duration = Duration::from_secs(5);

/// Benchmark service creation performance
fn service_creation_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("service_creation");
    group.measurement_time(BENCH_MEASUREMENT_TIME);
    group.sample_size(100);

    group.bench_function("create_service", |b| {
        b.to_async(&rt).iter(|| async {
            ServiceInfo::new(
                "test_service",
                "_http._tcp.local",
                8080,
                Some(vec![("version", "1.0"), ("description", "Test service")]),
            ).unwrap()
        });
    });

    group.bench_function("create_service_with_protocol", |b| {
        b.to_async(&rt).iter(|| async {
            ServiceInfo::new(
                "test_service",
                "_http._tcp.local",
                8080,
                Some(vec![("version", "1.0")]),
            ).unwrap().with_protocol_type(ProtocolType::Mdns)
        });
    });

    group.finish();
}

/// Benchmark service type creation performance
fn service_type_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("service_type");
    group.measurement_time(BENCH_MEASUREMENT_TIME);
    group.sample_size(1000);

    group.bench_function("create_service_type", |b| {
        b.to_async(&rt).iter(|| async {
            ServiceType::new("_http._tcp.local").unwrap()
        });
    });

    group.bench_function("create_upnp_service_type", |b| {
        b.to_async(&rt).iter(|| async {
            ServiceType::new("urn:schemas-upnp-org:service:ContentDirectory:1").unwrap()
        });
    });

    group.finish();
}

/// Benchmark configuration creation
fn config_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("config");
    group.measurement_time(BENCH_MEASUREMENT_TIME);
    group.sample_size(1000);

    group.bench_function("create_default_config", |b| {
        b.to_async(&rt).iter(|| async {
            DiscoveryConfig::default()
        });
    });

    group.bench_function("create_custom_config", |b| {
        b.to_async(&rt).iter(|| async {
            DiscoveryConfig::new()
                .with_timeout(Duration::from_secs(5))
                .with_max_retries(3)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    service_creation_benchmark,
    service_type_benchmark,
    config_benchmark
);
criterion_main!(benches);
