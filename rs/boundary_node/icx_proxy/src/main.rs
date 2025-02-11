use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Error};
use clap::{builder::ValueParser, crate_authors, crate_version, Parser};
use futures::try_join;
use hyper::Uri;
use itertools::iproduct;
use tracing::{error, Instrument};

mod canister_alias;
mod canister_id;
mod config;
mod domain_addr;
mod headers;
mod http_client;
mod logging;
mod metrics;
mod proxy;
mod validate;

use crate::{
    canister_alias::{parse_canister_alias, CanisterAlias},
    domain_addr::{parse_domain_addr, DomainAddr},
    metrics::{MetricParams, WithMetrics},
    validate::Validator,
};

#[derive(Parser)]
#[clap(
    version = crate_version!(),
    author = crate_authors!(),
    propagate_version = true,
)]
struct Opts {
    /// The address to bind to.
    #[clap(long, default_value = "127.0.0.1:3000")]
    address: SocketAddr,

    /// A list of mappings from domains to socket addresses for replica upstreams.
    /// Format: domain:addr
    #[clap(long, value_parser = ValueParser::new(parse_domain_addr))]
    replica_domain_addr: Vec<DomainAddr>,

    /// A list of domains that can be served. These are used for canister resolution.
    #[clap(long)]
    domain: Vec<String>,

    /// A list of mappings from canister names to canister principals.
    /// Format: name:principal
    #[clap(long, value_parser = ValueParser::new(parse_canister_alias))]
    canister_alias: Vec<CanisterAlias>,

    /// Whether or not to ignore `canisterId=` when locating the canister.
    #[clap(long)]
    ignore_url_canister_param: bool,

    /// The list of custom root HTTPS certificates to use to talk to the replica. This can be used
    /// to connect to an IC that has a self-signed certificate, for example. Do not use this when
    /// talking to the Internet Computer blockchain mainnet as it is unsecure.
    #[clap(long)]
    ssl_root_certificate: Vec<PathBuf>,

    /// Whether or not to fetch the root key from the replica back end. Do not use this when
    /// talking to the Internet Computer blockchain mainnet as it is unsecure.
    #[clap(long)]
    fetch_root_key: bool,

    /// Allows HTTPS connection to replicas with invalid HTTPS certificates. This can be used to
    /// connect to an IC that has a self-signed certificate, for example. Do not use this when
    /// talking to the Internet Computer blockchain mainnet as it is *VERY* unsecure.
    #[clap(long)]
    danger_accept_invalid_ssl: bool,

    /// Whether or not this is run in a debug context (e.g. errors returned in responses
    /// should show full stack and error details).
    #[clap(long)]
    debug: bool,

    /// The options for logging
    #[clap(flatten)]
    log: logging::LoggingOpts,

    /// The options for metrics
    #[clap(flatten)]
    metrics: metrics::MetricsOpts,
}

fn main() -> Result<(), anyhow::Error> {
    let Opts {
        address,
        replica_domain_addr,
        domain,
        canister_alias,
        ignore_url_canister_param,
        ssl_root_certificate,
        fetch_root_key,
        danger_accept_invalid_ssl,
        debug,
        log,
        metrics,
        ..
    } = Opts::parse();

    let _span = logging::setup(log);

    // Setup HTTP Client
    let client = http_client::setup(http_client::HttpClientOpts {
        ssl_root_certificates: ssl_root_certificate,
        danger_accept_invalid_ssl,
        domain_addrs: replica_domain_addr.clone(),
    })?;

    // Setup Metrics
    let (meter, metrics) = metrics::setup(metrics);

    // Setup Canister ID Resolver
    let dns_suffixes: Vec<String> = domain
        .iter()
        .flat_map(|domain| [domain.clone(), format!("raw.{domain}")])
        .collect();

    let dns_aliases: Vec<String> = iproduct!(canister_alias.iter(), domain.iter())
        .flat_map(|(CanisterAlias { id, principal }, domain)| {
            [
                format!("{id}.{domain}:{principal}"),
                format!("{id}.raw.{domain}:{principal}"),
            ]
        })
        .collect();

    let resolver = canister_id::setup(canister_id::CanisterIdOpts {
        dns_alias: dns_aliases,
        dns_suffix: dns_suffixes,
        ignore_url_canister_param,
    })?;

    // Setup Validator
    let validator = Validator::new();
    let validator = WithMetrics(validator, MetricParams::new(&meter, "validator"));

    // Setup Proxy
    let replica_uris: Vec<Uri> = replica_domain_addr
        .iter()
        .map(|v| {
            let uri = format!("https://{}:{}/", v.domain, v.addr.port());
            uri.parse::<Uri>().context("failed to parse uri")
        })
        .collect::<Result<_, Error>>()?;

    let proxy = proxy::setup(
        proxy::SetupArgs {
            resolver,
            validator,
            client,
        },
        proxy::ProxyOpts {
            address,
            replica_uris,
            debug,
            fetch_root_key,
        },
    )?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(10)
        .enable_all()
        .build()?;

    rt.block_on(
        async move {
            let v = try_join!(
                metrics.run().in_current_span(),
                proxy.run().in_current_span(),
            );
            if let Err(v) = v {
                error!("Runtime crashed: {v}");
                return Err(v);
            }
            Ok(())
        }
        .in_current_span(),
    )?;

    Ok(())
}
