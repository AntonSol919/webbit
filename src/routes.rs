
use std::{sync::Arc};
use crate::{
    reqtypes::{pkts_data::Pkts, Result, *},
    utils::{ *},
    Lk,
};
use anyhow::{Context};
use linkspace::{
    prelude::{LkHash, NetPkt, Point, PointExt ,NetPktExt, NetPktBox},
    runtime::{ lk_save_all, lk_get_hash, lk_get_all}, lk_process, lk_eval, point::lk_write,
};
use rocket::{
    data::{Capped, ToByteUnit},
    http::{ContentType, Status },
    response::{
        content::{RawHtml, RawJson},
        status::{Created, NotFound },
        Redirect,
    },
    tokio::{fs::File },
    *, fs::NamedFile,
};
use tokio::{io::{AsyncWriteExt }, task::{ spawn_blocking}};



pub fn routes() -> Vec<Route> {
    rocket::routes![
        index,
        
        favicon,
        eval,
        get_all,

        view_plain,
        view_with_upload,
        view_any,
        alts,
        save,
        save_pkts,
        quarantine,
        quarantine_blob,
        vouch,
        preview,
        query_json,
        query_html,
    ]
}
#[get("/favicon.ico",rank=0)]
fn favicon() -> (ContentType, &'static [u8]) {
    (ContentType::Icon,include_bytes!("./favicon.ico"))
}
/*
This endpoint exists for two reasons:
1. Its nice to have the ability to [#:anton:nl] and see if you agree on the result. 
2.User input WILL be eval'ed.
  The scope must be set such that this can not be a denial of service attack
  and it must not expose sensitive information.
*/
#[post("/eval?<hash>", data = "<data>")]
async fn eval(data:Data<'_>, hash: Option<Hash>, lk: &State<Lk>) -> Result<(ContentType,Vec<u8>)>{
    let expr = data.open(1.kibibytes()).into_string().await?;
    let bytes = match hash {
        Some(hash) => {
            let lk = lk.tlk();
            lk_get_hash(&lk, hash.0, &mut |p| lk_eval(&*expr, &p as &dyn NetPkt))?.context("hash not found")??
        },
        None => {
            // lk_eval uses the thread local lk - this should updates the lk's read transaction.
            let _lk = lk.tlk(); 

            // We manualy use abe::ctx  but this is shorter
            lk_eval(&expr,())?
        }
    };
    match String::from_utf8(bytes){
        Ok(b) => Ok((ContentType::Text,b.into_bytes())),
        Err(e) => Ok((ContentType::Bytes,e.into_bytes()))
    }
}

#[get("/<ipath..>?alts&<hash>",rank=2)]
async fn alts(_w:Webbit, ipath: AnyIPath,hash: Option<Hash>) -> Result<NotFound<RawHtml<String>>>{
    let alt_list = tokio::fs::read_to_string("./alts").await?;
    let mut st = format!("<ul>");
    use std::fmt::Write;
    let uri = uri!(view_any(ipath,hash)).to_string();
    for host in alt_list.lines(){
        let _ =write!(st,"<li><a href=\"{host}{uri}\">{host}</a></li>");
    }
    st +="</ul>";
    Ok(NotFound(RawHtml(st)))
}

// --- query functions start at rank 10 ( views start at rank 100)
#[get("/<_..>?pkts",rank=3)]
fn get_all(_w:Webbit, query: LkQuery<'_>, lk: &State<Lk>) -> Result<Vec<u8>>{
    let mut bufs = vec![];
    let lk = lk.tlk();
    lk_get_all(&lk, &query.0.query, &mut |p| {lk_write(p, false, &mut bufs).is_ok()})?;
    Ok(bufs)
}


#[get("/<_ipath..>", format = "text/html",rank=10)]
fn query_html(_w:Webbit, _ipath: AnyIPath,query:InfoQuery<'_>, lk: &State<Lk>) -> Result<RawHtml<String>> {
    query2html(&query.0.query, &lk.tlk())
}

#[get("/<_ipath..>", format = "application/json",rank=11)]
fn query_json(_w:Webbit, _ipath: AnyIPath,query: InfoQuery<'_>, lk: &State<Lk>) -> Result<RawJson<String>> {
    query2json(&query.0.query, &lk.tlk())
}

#[get("/<_ipath..>", format = "text/html",rank=12)]
async fn index(_w:Webbit, _ipath: AnyIPath,_tail:TailSlash) -> Result<RawHtml<File>,std::io::Error> {
    tokio::fs::File::open("./template/index.html").await.map(RawHtml)
}



// --- view function start at rank 100 

#[get("/<_..>?uploader",rank=100)]
async fn view_with_upload(w:Webbit, query:LkQuery<'_>, lk: &State<Lk>) -> Result<View> {
    _view(w,query.0, true, lk).await
}
#[get("/<_..>", rank = 110)]
async fn view_plain(w:Webbit, query:LkQuery<'_>,lk: &State<Lk>) -> Result<View> {
    _view(w,query.0, false, lk).await
}

#[derive(Responder)]
enum View {
    LkFile((ContentType, HeaderHash<Vec<u8>>)),
    Blob(HeaderHash<Vec<u8>>),
    Alts(Redirect),
    RealPath(Redirect),
    Template((Status,NamedFile)),
    #[response(status = 200)]
    TemplateMut(RawHtml<Vec<u8>>),
    #[response(status = 417)]
    ContentError(String),
}

// LkQuery contains the ipath and hash, but like this we can use the type checked  uri!(.. .. ) macro. 
#[get("/<ipath..>?<hash>",rank = 199)]
async fn view_any(_w:Webbit,ipath:AnyIPath,hash:Option<Hash>,q : LkQuery<'_>, lk: &State<Lk>) -> Result<View> {
    let q = q.0;
    let pkt = {read_pkt(q, lk.tlk())?};
    let r = match pkt {
        Some(Either::Left(data)) => {
            let ext = q.path.0.last().rsplit(|v| *v == b'.').next().unwrap_or(b"");
            match std::str::from_utf8(ext)
                .ok()
                .and_then(|v| ContentType::from_extension(v))
            {
                Some(c) => View::LkFile((c, data)),
                None => View::Blob(data),
            }
        }
        None => match hash {
            Some(hash) => View::Alts(Redirect::temporary(uri!(alts(ipath, Some(hash))))),
            None => View::Template((Status::NotFound,NamedFile::open("./template/no_editor.html").await?)),
        },
        Some(Either::Right(pkt)) => View::RealPath(Redirect::permanent(uri!(view_any(AnyIPath::new(pkt.get_ipath()), hash))))
    };
    Ok(r)
}

async fn _view(_w:Webbit, q: &ReqQuery, uploader: bool, lk: &State<Lk>) -> Result<View> {
    let pkt = {read_pkt(q, lk.tlk())? };
    let r = match pkt{
        Some(Either::Left(data)) if !uploader => View::LkFile((ContentType::HTML, data)),
        Some(Either::Left(HeaderHash(hash,data))) => match std::str::from_utf8(&data) {
            Ok(o) => match insert_html_header(
                &o,
                "<script id='webbitScript' src='/uploader.js'></script>",
            ) {
                Ok(h) => View::LkFile((ContentType::HTML, HeaderHash(hash,h.into_bytes()))),
                Err(e) => View::ContentError(e.to_string()),
            },
            Err(e) => View::ContentError(e.to_string()),
        },
        None => match q.hash {
            Some(hash) => View::Alts(Redirect::temporary(uri!(alts(q.path.clone().any(), Some(Hash::new(hash)))))),
            None if !uploader => View::Template((Status::NotFound,NamedFile::open("./template/html_editor.html").await?)),
            None => {
                // this is a bit convoluted, but it shows the user that ?uploader can be applied to any page.
                let editor = tokio::fs::read_to_string("./template/html_editor.html").await?;
                let editor = insert_html_header(&editor, "<script id='webbitScript' src='/uploader.js'></script>")
                    .context("can't inject the script into html_editor")?;

                View::TemplateMut(RawHtml(editor.into_bytes()))
            }
        },
        Some(Either::Right(pkt)) => View::RealPath(Redirect::permanent(uri!(view_any(AnyIPath::new(pkt.get_ipath()), Some(Hash::new(pkt.hash()))))))
    };
    Ok(r)
}

// --- 

#[derive(Responder)]
enum Upload{
    #[response(status = 400)]
    BadReq(&'static str),
    Created(Created<String>),
    #[response(status = 417)]
    NoVouch(&'static str),
}

#[post("/<ipath..>?pkts",format="bytes", data = "<pkts>",rank=300)]
async fn save_pkts(_w:Webbit, ipath: AnyIPath, pkts: Pkts<'_>) -> Result<Upload>{
    let (head, rest) = pkts.0.split_first().context("missing pkts")?;
    if !rest.iter().all(|p| p.is_datapoint()) { return Ok(Upload::BadReq("currently only a list of datapoints is accepted"))}
    match head.ipath(){
        None => return Ok(Upload::BadReq("first packet has no path")),
        Some(p) if p != &*ipath.0 =>return Ok(Upload::BadReq("using wrong path")),
        _ => {}
    }

    if head.pubkey().is_none() && pkts.0.len() <2 { return Ok(Upload::BadReq("a empty linkpoint is not accepted"))}
    write_quarantine(&pkts.0).await?;
    if head.pubkey().is_none(){
        let qh = QUARANTINE.get().unwrap();
        let qh = qh[0].to_absolute("http", qh).unwrap();
        let hash = head.hash();
        return Ok(Upload::Created(Created::new(uri!(qh,quarantine(Hash::new(hash),None::<String>)).to_string())))
    };
    vouch_cmd(pkts.0, head.hash()).await
}

// The 'data' param exists mostly to avoid botnets spamming post requests.
#[post("/<ipath..>?data", data = "<file>",rank=301)] 
async fn save(_w:Webbit, ipath: AnyIPath, file: Capped<Vec<u8>>) -> Result<Upload>{
    let file = file.into_inner();
    if ipath.0.last().ends_with(b".html") {
        let insert : anyhow::Result<_> = try {
            let file = std::str::from_utf8(&file)?;
            tracing::trace!("Body is {file:#?}");
            insert_html_header(file, "")?
        };
        if let Err(_e) = insert {
            return Ok(Upload::BadReq("can't inject upload script into html - refused"))
        }
    }
    let mut pkts = vec![];
    use linkspace::prelude::{consts::*, point::*, *};
    
    let mut ptr = file.as_slice();
    const SPLIT: [u8; 18] = *b"<!--SPLIT_POINT-->";
    let manual_split_points = std::iter::from_fn(move || {
        if ptr.is_empty() {
            return None;
        }
        
        let max = &ptr[..MAX_DATA_SIZE.min(ptr.len())];
        match max.array_windows().position(|v| *v == SPLIT) {
            Some(i) => {
                let (head, rest) = ptr.split_at(i);
                ptr = rest;
                Some(head)
            }
            None => {
                ptr = &ptr[max.len()..];
                Some(max)
            }
        }
    });
    for chunk in manual_split_points.flat_map(|i| i.chunks(MAX_DATA_SIZE)) {
        pkts.push(lk_datapoint_ref(&chunk).unwrap());
    }
    let links: Vec<Link> = pkts.iter().map(|p| Link::new("data", p.hash())).collect();
    let linkpoint = lk_linkpoint_ref(&[], ab(b"webbit"), PUBLIC, &ipath.0, &links, None)?;
    pkts.insert(0, linkpoint);
    tracing::info!(len=pkts.len(),"total packets");
    write_quarantine(&pkts).await?;
    let qh = QUARANTINE.get().unwrap();
    let qh = qh[0].to_absolute("http", qh).unwrap();
    let hash = linkpoint.hash();
    return Ok(Upload::Created(Created::new(uri!(qh,quarantine(Hash::new(hash),None::<String>)).to_string())))
}


#[get("/<ipath..>?preview&<hash>&<unsafe>")]
async fn preview(_q:Webbit,ipath:HtmlIPath, hash: Hash, r#unsafe: Option<bool>) -> Result<(Status, RawHtml<String>)> {
    let pktbytes = tokio::fs::read(format!("./quarantine/{}", hash.0)).await?;
    let mut it = iter_pkts_unchecked(&pktbytes);
    let linkpoint = it.next().context("quarantine error")?;
    if linkpoint.get_ipath() != &*ipath.0 { return Ok((Status::BadRequest,RawHtml("path does not match the packet".to_string())));}
    let mut bytes = vec![];
    it.for_each(|p| bytes.extend_from_slice(p.data()));
    let st = String::from_utf8(bytes)?;
    if r#unsafe.unwrap_or(false) {
        return Ok((Status::Ok, RawHtml(st)));
    }
    else { Ok((Status::BadRequest,RawHtml("this instance requires you add &unsafe to the query".to_string())))}
    /*
    //let clean = ammonia::clean(&st);
    let st = HTML_PREFIX
        .into_iter()
        .chain(["</head><body>", &clean, "</body></html>"])
        .collect();
    Ok((Status::Ok, RawHtml(st)))
    */
}


#[get("/<hash>?pkts",rank=200)]
async fn quarantine_blob(_q:Quarantine,hash: Hash) -> Option<File> {
    tokio::fs::File::open(format!("./quarantine/{}", hash.0))
        .await
        .ok()
}

#[get("/<hash>?<back>",rank=201)]
async fn quarantine(_q:Quarantine, hash: Hash,back:Option<String>) -> Either<RawHtml<File>,&'static str>{
    let _ = back;
    let _ = hash;
    match tokio::fs::File::open(format!("./template/quarantine.html")).await.ok(){
        Some(v) => Either::Left(RawHtml(v)),
        None => Either::Right("can't open"),
    }
}

#[post("/vouch/<hash>", data = "<pkts>")]
async fn vouch(
    _q:Quarantine,
    hash: Hash,
    pkts: Pkts<'_>,
) -> Result<Upload> {
    // this should prob be request guards.
    let pkts = pkts.0;
    if pkts.len() != 1 {
        return Ok(Upload::BadReq("expected 1 packet"));
    }

    let keypoint = &pkts[0];
    use linkspace::prelude::{PktFmt};
    if !keypoint.is_keypoint() {
        return Ok(Upload::BadReq("vouching is done with a keypoint"));
    }

    let pktbytes = tokio::fs::read(format!("./quarantine/{}", hash.0)).await?;
    let mut it = iter_pkts_unchecked(&pktbytes);
    let linkpoint = it.next().context("quarantine error")?;
    tracing::trace!(linkpoint=%PktFmt(&linkpoint),keypoint=%PktFmt(&keypoint),"checking equality");
    if linkpoint.group() != keypoint.group()
        || linkpoint.domain() != keypoint.domain()
        || linkpoint.path() != keypoint.path()
        || linkpoint.links() != keypoint.links()
    {
        return Ok(Upload::BadReq("your packet does not match the original"));
    }
    // This is dumb
    let mut pkts = vec![pkts[0].clone()];
    pkts.extend(it);
    let arc = Arc::from(pkts);
    vouch_cmd(&arc, hash.0).await
}

async fn vouch_cmd(pkts: &Arc<[NetPktBox]>, quarantine: LkHash) -> Result<Upload> {
    use std::process::Stdio;
    use tokio::process::Command;
    let keyp = &pkts[0];
    assert!(keyp.is_keypoint());
    let mut cmd = Command::new("./vouch")
        .arg(format!("{}", keyp.get_pubkey()))
        .arg(format!("./quarantine/{}", quarantine))
        .stdin(Stdio::piped())
        .spawn()?;
    let mut stdin = cmd.stdin.take().unwrap();
    let _write = stdin.write_all(keyp.as_netpkt_bytes()).await;
    let _shutdown = stdin.shutdown().await?;
    std::mem::drop(stdin);
    let result = cmd.wait().await?;
    eprintln!("Exec : {result:#?}");
    if !result.success(){
        return Ok(Upload::NoVouch("Computer says no"));
    }
    let pkt_arc = pkts.clone();
    spawn_blocking(move || -> anyhow::Result<()>{
        let refs = pkt_arc
            .iter()
            .map(|p| &*p as &dyn NetPkt)
            .collect::<Vec<_>>();
        let local_lk = Lk.tlk();
        let new_pkts = lk_save_all(&local_lk, &refs)?;
        let txn_head = lk_process(&local_lk).get();
        tracing::info!(new_pkts,txn_head, "save ok");
        Ok(())
    }).await??;
    
    let path = AnyIPath::new(keyp.get_ipath()).cast();

    let webbit = WEBBIT.get().unwrap();
    let webbit = webbit[0].to_absolute("http",webbit).unwrap();
    Ok(Upload::Created(Created::new(uri!(webbit,view_any(path, Some(Hash::new(keyp.hash())))).to_string())))
}

