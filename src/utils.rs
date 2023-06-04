

use anyhow::Context;
use linkspace::consts::PUBLIC_GROUP_PKT;
use linkspace::point::lk_read;
use linkspace::prelude::*;
use linkspace::runtime::{lk_get_hash,lk_get_all};
use rocket::Either;
use rocket::response::content::{RawHtml, RawJson};
use crate::reqtypes::*;


pub fn ok<A,T,E>(ff:impl FnOnce(A) -> Result<T,E>) -> impl FnOnce(A) -> Option<T> { |v| (ff)(v).ok()}

/// Some(Right(pkt)) means a packet was found under the wrong ipath. 
pub fn read_pkt(ipath: &IPath, hash: Option<Hash>,lk:linkspace::Linkspace) -> anyhow::Result<Option<Either<Vec<u8>,NetPktBox>>>{
    use linkspace::prelude::*;
    use linkspace::runtime::*;
    let linkpkt = match hash {
        None => {
            let query = lk_query_push(lk_query(&crate::Q), "path", "=", ipath.spath_bytes())?;
            let query = lk_query_push(query, "i_branch", "=", &[0,0,0,0])?;
            let mut r = PUBLIC_GROUP_PKT.as_netbox();
            lk_get_all(&lk, &query, &mut |pkt| {
                if pkt.get_create_stamp() > r.get_create_stamp() {
                    r = pkt.as_netbox()
                }
                return false
            })?;
            if r.hash() == PUBLIC { return Ok(None)}
            r
        },
        Some(hash) => {
            let pkt = match lk_get_hash(&lk,hash.0, &mut |p| p.as_netbox())?{
                Some(p) => p,
                None => return Ok(None),
            };
            if pkt.get_ipath() != ipath{
                return Ok(Some(Either::Right(pkt)))
            }
            pkt
        }
    };
    let data = collect(&lk,&linkpkt)?;
    Ok(Some(Either::Left(data)))
}

pub fn collect(lk: &Linkspace, pkt:&dyn NetPkt) -> anyhow::Result<Vec<u8>>{
    let mut data = vec![];
    let data_links = pkt.get_links().iter().filter(|l| l.tag == ab(b"data"));
    for el in data_links{
        lk_get_hash(&lk, el.ptr, &mut |pkt| data.extend_from_slice(pkt.data()))?.context("missing data pkt")?;
    }
    Ok(data)
}



pub fn iter_pkts_unchecked(mut ptr:&[u8]) -> impl Iterator<Item= NetPktBox> + '_{
    std::iter::from_fn(move ||{
        if ptr.is_empty() { return None};
        let (pkt,rest) = linkspace::point::lk_read_unchecked(&ptr, false).expect("a private packet got in?");
        ptr = rest;
        Some(pkt)
    })
}

pub fn try_iter_pkts(mut ptr:&[u8]) -> impl Iterator<Item=Result<NetPktBox,linkspace::prelude::PktError>> + '_{
    std::iter::from_fn(move ||{
        if ptr.is_empty() { return None};
        match lk_read(&ptr, false){
            Ok((pkt,rest)) => {
                ptr =rest;
                Some(Ok(pkt))
            },
            Err(e) => {
                ptr = &[];
                Some(Err(e))
            }
        }
    })
}

pub fn query2html(query: Query, lk: &Linkspace) -> Result<RawHtml<String>> {
    let mut string = format!("<ol>");
    use std::fmt::Write;
    lk_get_all(&lk, &query, &mut |pkt: &dyn NetPkt| {
        let delta = lk_eval("[create/s:delta]", pkt)
            .ok()
            .and_then(ok(String::from_utf8))
            .unwrap_or_else(|| "???".into());
        let create = pkt.get_create_stamp().get();
        let hash = pkt.hash_ref();
        let hash_mini = hash.b64_mini();
        let pathname = pkt.path().and_then(ipath_uri_display).map(|o|format!("/webbit/{o}"))
            .unwrap_or(String::new());
        let href = format!("href=\"{pathname}?hash={hash}\"");

        let (pubkey_attr,pubkey_str)= match pkt.pubkey() {
            Some(k) => (format!("pubkey=\"{k}\""),format!(" -- &( {} )",k.b64_mini())),
            None => (String::new(),String::new()),
        };
        let _ = write!(
            &mut string,
            "<li><a {href} {pubkey_attr} create=\"{create}\">{pathname} {hash_mini} {delta} {pubkey_str} </a></li>",
        );

        false
    })?;
    string += "</ol>";
    Ok(RawHtml(string))
}

pub fn query2json(query: Query, lk: &Linkspace) -> Result<RawJson<String>> {
    let mut string = "[ ".to_string();
    use std::fmt::Write;
    lk_get_all(&lk, &query, &mut |pkt: &dyn NetPkt| {
        let hash = format!(" \"hash\":\"{}\"",pkt.hash());
        let create = format!(" \"create\":\"{}\"",pkt.get_create_stamp());
        let pubkey = match pkt.pubkey() {
            Some(p) => format!(", \"pubkey\":\"{p}\""),
            None => String::new()
        };
        let path = match ipath_uri_display(pkt.get_ipath()) {
            Some(p) => format!(", \"path\":\"{}\"",p),
            _ => String::new()
        };
        let _ = write!(&mut string, "{{ {hash},{create} {pubkey} {path} }},");
        false
    })?;
    string.pop();
    string.push(']');
    Ok(RawJson(string))
}

pub static HTML_PREFIX: [&str; 3] = [
    "<!DOCTYPE html>",
    r#"<html xmlns="http://www.w3.org/1999/xhtml">"#,
    "<head>",
];

pub fn insert_html_header(mut data: &str, head: &str) -> anyhow::Result<String> {
    for el in HTML_PREFIX {
        let len = data.as_bytes().len();
        data = data.trim_start_matches(|c: char| c.is_ascii_whitespace());
        anyhow::ensure!(data.as_bytes().len() + 8 > len, "too much whitespace");
        data = data
            .strip_prefix(el)
            .with_context(|| anyhow::anyhow!("Missing {el} - got {}", data))?;
    }
    Ok(HTML_PREFIX.into_iter().chain([head, data]).collect())
}
