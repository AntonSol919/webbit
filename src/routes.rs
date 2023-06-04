use std::{io::Write, time::Duration};

use crate::{
    reqtypes::{pkts_data::Pkts, Result, *},
    utils::{ *},
    Lk,
};
use anyhow::{anyhow, Context};
use linkspace::{
    lk_query, lk_query_push,
    prelude::{now, LkHash, NetPkt, NetPktPtr, Point, PointExt},
    runtime::{ lk_save_all},
};
use rocket::{
    data::Capped,
    form::Form,
    http::{ContentType, Status},
    response::{
        content::{RawHtml, RawJson},
        status::{Created, NotFound, BadRequest},
        Redirect,
    },
    tokio::{fs::File, task::spawn_blocking},
    *, fs::NamedFile,
};
use tokio::io::AsyncWriteExt;



pub fn routes() -> Vec<Route> {
    rocket::routes![
        view_plain,
        view_with_upload,
        view_any,
        alts,
        save,
        quarantine,
        quarantine_blob,
        vouch,
        preview,
        tree_json,
        tree_html,
        list_json,
        list_html,
        compromised,
        get_compromised
    ]
}

#[get("/<ipath..>?alts&<hash>",rank=1)]
async fn alts(_w:Webbit, ipath: AnyIPath,hash: Hash) -> Result<NotFound<RawHtml<String>>>{
    let alt_list = tokio::fs::read_to_string("./alts").await?;
    let mut st = format!("<ul>");
    use std::fmt::Write;
    let uri = uri!(view_any(ipath,Some(hash))).to_string();
    for host in alt_list.lines() {
        let _ =write!(st,"<li><a href=\"{host}{uri}\">{host}</a></li>");
    }
    st +="</ul>";
    Ok(NotFound(RawHtml(st)))
}

// --- query functions start at rank 10 ( views start at rank 100)

#[get("/<ipath..>?tree", format = "text/html",rank=10)]
fn tree_html(_w:Webbit, ipath: AnyIPath, lk: &State<Lk>) -> Result<RawHtml<String>> {
    let q = lk_query_push(lk_query(&*crate::Q), "prefix", "=", ipath.0.spath_bytes())?;
    query2html(q, &lk.tlk())
}
#[get("/<ipath..>?list", format = "text/html",rank=11)]
fn list_html(_w:Webbit, ipath: AnyIPath, lk: &State<Lk>) -> Result<RawHtml<String>> {
    let q = lk_query_push(lk_query(&*crate::Q), "path", "=", ipath.0.spath_bytes())?;
    query2html(q, &lk.tlk())
}

#[get("/<ipath..>?list", format = "application/json",rank=12)]
fn list_json(_w:Webbit, ipath: AnyIPath, lk: &State<Lk>) -> Result<RawJson<String>> {
    let q = lk_query_push(lk_query(&*crate::Q), "path", "=", ipath.0.spath_bytes())?;
    query2json(q, &lk.tlk())
}
#[get("/<ipath..>?tree", format = "application/json",rank=13)]
fn tree_json(_w:Webbit, ipath: AnyIPath, lk: &State<Lk>) -> Result<RawJson<String>> {
    let q = lk_query_push(lk_query(&*crate::Q), "prefix", "=", ipath.0.spath_bytes())?;
    query2json(q, &lk.tlk())
}


// --- view function start at rank 100 

#[get("/<ipath..>?<hash>&uploader",rank=100)]
async fn view_with_upload(w:Webbit, ipath: HtmlIPath, hash: Option<Hash>, lk: &State<Lk>) -> Result<View> {
    _view(w,ipath, hash, true, lk).await
}
#[get("/<ipath..>?<hash>", rank = 110)]
async fn view_plain(w:Webbit, ipath: HtmlIPath, hash: Option<Hash>, lk: &State<Lk>) -> Result<View> {
    _view(w,ipath, hash, false, lk).await
}

#[derive(Responder)]
enum View {
    LkFile((ContentType, Vec<u8>)),
    Blob(Vec<u8>),
    Alts(Redirect),
    RealPath(Redirect),
    Template((Status,NamedFile)),
    #[response(status = 417)]
    ContentError(String),
}
#[get("/<ipath..>?<hash>",rank = 199)]
async fn view_any(_w:Webbit, ipath: AnyIPath, hash: Option<Hash>, lk: &State<Lk>) -> Result<View> {
    let pkt = {read_pkt(&ipath.0, hash, lk.tlk())?};
    let r = match pkt {
        Some(Either::Left(data)) => {
            let ext = ipath.0.last().rsplit(|v| *v == b'.').next().unwrap_or(b"");
            match std::str::from_utf8(ext)
                .ok()
                .and_then(|v| ContentType::from_extension(v))
            {
                Some(c) => View::LkFile((c, data)),
                None => View::Blob(data),
            }
        }
        None => match hash {
            Some(hash) => View::Alts(Redirect::temporary(uri!(alts(ipath.any(), hash)))),
            None => View::Template((Status::NotFound,NamedFile::open("./template/no_editor.html").await?)),
        },
        //Some(Either::Right(_pkt)) => View::Template((Status::NotFound,NamedFile::open("./template/wrong_path.html").await?)),
        Some(Either::Right(pkt)) => View::RealPath(Redirect::permanent(uri!(view_any(AnyIPath::new(pkt.get_ipath()), hash))))
    };
    Ok(r)
}

async fn _view(_w:Webbit, ipath: HtmlIPath, hash: Option<Hash>, uploader: bool, lk: &State<Lk>) -> Result<View> {
    let pkt = {read_pkt(&ipath.0, hash, lk.tlk())? };
    let r = match pkt{
        Some(Either::Left(data)) if !uploader => View::LkFile((ContentType::HTML, data)),
        Some(Either::Left(data)) => match std::str::from_utf8(&data) {
            Ok(o) => match insert_html_header(
                &o,
                "<script id='webbitScript' src='/uploader.js'></script>",
            ) {
                Ok(h) => View::LkFile((ContentType::HTML, h.into_bytes())),
                Err(e) => View::ContentError(e.to_string()),
            },
            Err(e) => View::ContentError(e.to_string()),
        },
        None => match hash {
            Some(hash) => View::Alts(Redirect::temporary(uri!(alts(ipath.any(), hash)))),
            None if !uploader => View::Template((Status::NotFound,NamedFile::open("./template/html_editor.html").await?)),
            None => {
                // this is a bit convoluted, but it shows the user that ?uploader can be applied to any page.
                let editor = tokio::fs::read_to_string("./template/html_editor.html").await?;
                let editor = insert_html_header(&editor, "<script id='webbitScript' src='/uploader.js'></script>")
                    .context("can't inject the script into html_editor")?;

                View::LkFile((ContentType::HTML,editor.into_bytes()))
            }
        },
        //Some(Either::Right(_pkt)) => View::Template((Status::NotFound,NamedFile::open("./template/wrong_path.html").await?)),
        Some(Either::Right(pkt)) => View::RealPath(Redirect::permanent(uri!(view_any(AnyIPath::new(pkt.get_ipath()), hash))))
    };
    Ok(r)
}

// --- 


#[get("/blob?<hash>")]
async fn quarantine_blob(_q:Quarantine,hash: Hash) -> Option<File> {
    tokio::fs::File::open(format!("./quarantine/{}", hash.0))
        .await
        .ok()
}

#[get("/html?<hash>&<back>")]
async fn quarantine(_q:Quarantine, hash: Hash,back:Option<String>) -> Option<RawHtml<File>> {
    let _ = back;
    let _ = hash;
    tokio::fs::File::open(format!("./template/quarantine.html"))
        .await
        .ok()
        .map(RawHtml)
}

#[post("/vouch?<hash>", data = "<pkts>")]
async fn vouch(
    _q:Quarantine,
    hash: Hash,
    pkts: Pkts<'_>,
    lk: &State<Lk>,
) -> Result<Either<(Status, &'static str), Created<String>>> {
    // this should be guards i think?
    let pkts = pkts.0;
    if pkts.len() != 1 {
        return Ok(Either::Left((Status::BadRequest, "expected 1 packet")));
    }

    let keypoint = &pkts[0];
    use linkspace::prelude::NetPktExt;
    let kp_hash = keypoint.hash();

    if !keypoint.is_keypoint() {
        return Ok(Either::Left((Status::BadRequest, "expected keypoint")));
    }

    let pktbytes = tokio::fs::read(format!("./quarantine/{}", hash.0)).await?;
    let mut it = iter_pkts_unchecked(&pktbytes);
    let linkpoint = it.next().context("quarantine error")?;
    if linkpoint.group() != keypoint.group()
        || linkpoint.domain() != keypoint.domain()
        || linkpoint.path() != keypoint.path()
        || linkpoint.links() != keypoint.links()
    {
        return Ok(Either::Left((Status::BadRequest,"your packet does not match the original")));
    }
    if keypoint.get_create_stamp().get() > now().get() + Duration::from_secs(600).as_micros() as u64
        || keypoint.get_create_stamp() < linkpoint.get_create_stamp()
    {
        return Ok(Either::Left((Status::BadRequest,"we're more than 10 minutes out of sync?")));
    }

    if !vouch_cmd(keypoint, hash.0).await? {
        return Ok(Either::Left((Status::Forbidden, "Computer says no")));
    }
    let pkts = it.collect::<Vec<_>>();
    let refs = pkts
        .iter()
        .map(|p| &*p as &dyn NetPkt)
        .chain(Some(&*keypoint as &dyn NetPkt))
        .collect::<Vec<_>>();
    lk_save_all(&lk.tlk(), &refs)?;

    let path = AnyIPath::new(linkpoint.get_ipath()).cast();

    let webbit = WEBBIT.get().unwrap();
    let webbit = webbit[0].to_absolute("http",webbit).unwrap();
   
    Ok(Either::Right(Created::new(
        uri!(webbit,view_plain(path, Some(Hash(kp_hash)))).to_string(),
    )))
}

pub async fn vouch_cmd(keyp: &NetPktPtr, quarantine: LkHash) -> anyhow::Result<bool> {
    use std::process::Stdio;
    use tokio::process::Command;
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
    Ok(result.success())
}

#[post("/<ipath..>", data = "<file>")]
async fn save(_w:Webbit, ipath: AnyIPath, file: Capped<Vec<u8>>) -> Result<Either<Created<String>,BadRequest<String>>>{
    let file = file.into_inner();
    if ipath.0.last().ends_with(b".html") {
        let insert : anyhow::Result<_> = try {
            let file = std::str::from_utf8(&file)?;
            insert_html_header(file, "")?
        };
        if let Err(e ) = insert {
            return Ok(Either::Right(BadRequest(Some(format!("the html you're submitting is invalid. {e:?}")))))
        }
    }
    let path = ipath.0.clone();
    let hash = spawn_blocking(move || -> anyhow::Result<LkHash> {
        let mut points = vec![];
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
            points.push(lk_datapoint_ref(&chunk).unwrap());
        }
        let links: Vec<Link> = points.iter().map(|p| Link::new("data", p.hash())).collect();
        let linkpoint = lk_linkpoint_ref(&[], ab(b"webbit"), PUBLIC, &path, &links, None)?;
        let path = format!("./quarantine/{}", linkpoint.hash());
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| anyhow!("opening {path}"))?;
        let mut slices = linkpoint
            .byte_segments()
            .io_slices()
            .into_iter()
            .chain(points.iter().flat_map(|p| p.byte_segments().io_slices()))
            .collect::<Vec<_>>();
        file.write_all_vectored(&mut slices)
            .context("write_vectored")?;
        Ok(linkpoint.hash())
    })
    .await??;
    let r : Option<String> = None;
    let qh = QUARANTINE.get().unwrap();
    let qh = qh[0].to_absolute("http", qh).unwrap();
    Ok(Either::Left(Created::new(uri!(qh,quarantine(Hash(hash),r)).to_string())))
}


#[get("/preview?<hash>&<unsafe>")]
async fn preview(_q:Quarantine,hash: Hash, r#unsafe: bool) -> Result<(Status, RawHtml<String>)> {
    let pktbytes = tokio::fs::read(format!("./quarantine/{}", hash.0)).await?;
    let mut it = iter_pkts_unchecked(&pktbytes);
    let _linkpoint = it.next().context("quarantine error")?;
    let mut bytes = vec![];
    it.for_each(|p| bytes.extend_from_slice(p.data()));
    let st = String::from_utf8(bytes)?;
    if r#unsafe {
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

#[derive(FromForm)]
struct FakeLogin {
    username: Capped<String>,
    password: Capped<String>,
}
#[get("/if_you_see_this_on_the_network_you_are_compromised.html")]
fn get_compromised(_q:Quarantine) -> Either<&'static str, RawHtml<String>> {
    compromised(_q,None)
}

#[post(
    "/if_you_see_this_on_the_network_you_are_compromised.html",
    data = "<login>"
)]
fn compromised(_q:Quarantine,login: Option<Form<FakeLogin>>) -> Either<&'static str, RawHtml<String>> {
    match login {
        None => Either::Left(
            r#"Little Bobby Tables>";'\0 && touch $HOME/i_am_a_dumbass' ); DROP TABLE Student; --"#,
        ),
        Some(login) => Either::Right(RawHtml(format!(
            "The key '{}' is compromised {}",
            *login.username, *login.password
        ))),
    }
}
