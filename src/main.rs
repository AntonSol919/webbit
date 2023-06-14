#![feature(try_blocks,lazy_cell,thread_local,write_all_vectored,array_windows,iterator_try_collect,str_split_whitespace_remainder)]

use std::{sync::LazyLock };

use linkspace::{lk_query_parse, Query, lk_query, lk_process};
use rocket::{launch, fs::FileServer, http::uri::{Authority, Host}};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use crate::reqtypes::{QUARANTINE, WEBBIT};
pub mod reqtypes;
pub mod routes;
pub mod utils;

/*
A linkspace runtime is always thread local and !Send.
Here we auto spawn it  whenever a thread requires it. 
The Lk struct is a managed state for rocket.
It ensures we use lk_process to update our read transaction.
*/
#[thread_local]
static LOCAL_LK: LazyLock<linkspace::Linkspace> = LazyLock::new(|| linkspace::lk_open(None,true).unwrap());
pub struct Lk;
impl Lk {
    pub fn tlk(&self) -> linkspace::Linkspace{
        let i = lk_process(&LOCAL_LK);
        tracing::info!("update lk tx to {i}");
        LOCAL_LK.clone()
    }
}

pub static Q : std::sync::LazyLock<Query> = std::sync::LazyLock::new(||{
    lk_query_parse(lk_query(&linkspace::Q), &[
        "domain:=:webbit",
        "group:=:[#:pub]"
    ], ()).unwrap()
});


#[launch]
fn rocket() -> _ {
    

    println!("Init");
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env().unwrap();
    println!("Env {env_filter:?}");
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init().unwrap();
    println!("Using {:?}",linkspace::runtime::lk_info(&LOCAL_LK).dir);
    tracing::info!("Tracing OK!");
    
    std::fs::create_dir_all("./quarantine").unwrap();
    let rocket =  rocket::build()
        .manage(Lk)
        .mount("/", FileServer::from("static/").rank(0))
        .mount("/",  routes::routes() );
    let figment = rocket.figment();
    let _ = QUARANTINE.get_or_init(|| {
        let address : Vec<String> = figment.extract_inner("quarantine_domain").unwrap();
        address.into_iter()
            .map(|i| Host::new(Authority::parse_owned(i).expect("invalid authority in quarantine_domain")))
            .inspect(|e| println!("Quarantine: {e:?}"))
            .collect()
    });
    let _= WEBBIT.get_or_init(|| {
        let address : Vec<String> = figment.extract_inner("webbit_domain").unwrap();
        address.into_iter()
            .map(|i| Host::new(Authority::parse_owned(i).expect("invalid authority in webbit_domain")))
            .inspect(|e| println!("Webbit: {e:?}"))
            .collect()
    });

    rocket 
}
