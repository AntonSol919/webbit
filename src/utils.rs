

use std::borrow::Cow;
use std::path::PathBuf;

use anyhow::Context;
use linkspace::prelude::*;
use linkspace::runtime::{lk_get_hash,lk_get_all};
use rocket::Either;
use rocket::response::content::{RawHtml, RawJson};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::reqtypes::*;


pub fn ok<A,T,E>(ff:impl FnOnce(A) -> Result<T,E>) -> impl FnOnce(A) -> Option<T> { |v| (ff)(v).ok()}

/// Some(Right(pkt)) means a packet was found under the wrong ipath. 
pub fn read_pkt(q:&ReqQuery,lk:linkspace::Linkspace) -> anyhow::Result<Option<Either<HeaderHash<Vec<u8>>,NetPktBox>>>{
    use linkspace::prelude::*;
    use linkspace::runtime::*;
    let linkpkt = match q.hash {
        None => {
            let query = lk_query_push(lk_query(&q.query), "i_branch", "=", &[0,0,0,0])?;
            let mut hash = PRIVATE; // [0;32]
            let mut stamp = Stamp::ZERO;
            // Our ibranch=0 means we only check the first of every 'branch', i.e.  uniq (path,pubkey) pairs.
            lk_get_all(&lk, &query, &mut |pkt| {
                if *pkt.get_create_stamp() > stamp{
                    stamp = *pkt.get_create_stamp();
                    hash = pkt.hash();
                }
                return false
            })?;
            if hash == PRIVATE { return Ok(None)}
            lk_get_hash(&lk, hash, &mut |o| o.as_netbox())?.unwrap()
        },
        Some(hash) => {
            let pkt = match lk_get_hash(&lk,hash, &mut |p| p.as_netbox())?{
                Some(p) => p,
                None => return Ok(None),
            };
            if pkt.get_ipath() != &*q.path.0{
                return Ok(Some(Either::Right(pkt)))
            }
            pkt
        }
    };
    let data = collect(&lk,&linkpkt)?;
    Ok(Some(Either::Left(HeaderHash(linkpkt.hash(),data))))
}

pub fn collect(lk: &Linkspace, pkt:&dyn NetPkt) -> anyhow::Result<Vec<u8>>{
    let mut data = vec![];
    let data_links = pkt.get_links().iter().filter(|l| l.tag == ab(b"data"));
    for el in data_links{
        lk_get_hash(&lk, el.ptr, &mut |pkt| data.extend_from_slice(pkt.data()))?.context("missing data pkt")?;
    }
    Ok(data)
}



pub fn iter_pkts_unchecked_alligned<'o>(ptr:&'o [u8]) -> impl Iterator<Item= &'o NetPktPtr> + '_{
    assert!(ptr.as_ptr().align_offset(8) == 0 );
    iter_pkts_unchecked(ptr).map(|o| match o {
        Cow::Borrowed(o) => o,
        Cow::Owned(_) => unreachable!(),
    })
}
pub fn iter_pkts_unchecked(mut ptr:&[u8]) -> impl Iterator<Item= Cow<NetPktPtr>> + '_{
    std::iter::from_fn(move ||{
        if ptr.is_empty() { return None};
        let (pkt,rest) = linkspace::point::lk_read_unchecked(&ptr, false).expect("a private packet got in?");
        ptr = rest;
        Some(pkt)
    })
}

pub fn try_iter_pkts(mut ptr:&[u8]) -> impl Iterator<Item=Result<Cow<NetPktPtr>,linkspace::prelude::PktError>> + '_{
    std::iter::from_fn(move ||{
        if ptr.is_empty() { return None};
        match linkspace::point::lk_read(&ptr, false){
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

pub fn query2html(query: &Query, lk: &Linkspace) -> Result<RawHtml<String>> {
    let mut string = format!("<ol>");
    use std::fmt::Write;
    lk_get_all(&lk, query, &mut |pkt: &dyn NetPkt| {
        let delta = lk_eval("[create/s:delta]", pkt)
            .ok()
            .and_then(ok(String::from_utf8))
            .unwrap_or_else(|| "???".into());
        let create = pkt.get_create_stamp().get();
        let hash = pkt.hash_ref();
        let hash_mini = hash.b64_mini();
        let pathname = pkt.path().and_then(ipath_uri_display).unwrap_or(String::new());
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

pub fn query2json(query: &Query, lk: &Linkspace) -> Result<RawJson<String>> {
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

pub async fn write_quarantine(pkts:&[impl NetPkt]) -> anyhow::Result<PathBuf>{
    let path = format!("./quarantine/{}", pkts[0].hash());
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .with_context(|| anyhow::anyhow!("opening {path}"))?;
    if file.is_write_vectored(){
        tracing::debug!("using write_vectored");
        let mut slices = pkts.iter().flat_map(|p| p.byte_segments().io_slices().into_iter().filter(|v| !v.is_empty()))
            .collect::<Vec<_>>();
        // manual write_all_vectored - this dance is prob not worth it atm.
        let mut slices :&mut [std::io::IoSlice] = slices.as_mut();
        while !slices.is_empty(){
            let mut bytes = file.write_vectored(&slices).await?;   
            while bytes != 0 {
                if bytes >= slices[0].len(){
                    bytes -= slices[0].len();
                    slices = &mut slices[1..];
                } else {
                    slices[0].advance(bytes);
                    bytes = 0;
                }
            }
        }
    }else {
        let bytes = {
            let size = pkts.iter().map(|v|v.size() as usize).sum();
            let mut bytes = vec![0;size];
            let mut dest = bytes.as_mut_ptr();
            for p in pkts {
                dest = unsafe { p.byte_segments().write_segments_unchecked(dest)};
            }
            bytes
        };
        file.write_all(&bytes).await?;
    }
    file.flush().await?;
    Ok(path.into())
}
