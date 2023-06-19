use anyhow::{bail, Context};
use linkspace::{ prelude::*, query::PredicateType};
use rocket::{
    form::{DataField, FromFormField, ValueField},
    http::{
        impl_from_uri_param_identity,
        uri::{
            self,
            fmt::{FromUriParam, UriDisplay},
            Host,
        },
        Header, Status,
    },
    outcome::IntoOutcome,
    request::{FromRequest, Outcome, FromParam},
};
use std::{marker::PhantomData, sync::OnceLock};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct B64b32(pub B64<[u8; 32]>);
impl B64b32 {
    pub fn new(bytes: impl AsRef<[u8; 32]>) -> Self {
        B64b32(B64(*bytes.as_ref()))
    }
}
pub type Hash = B64b32;
pub type Pubkey = B64b32;
#[rocket::async_trait]
impl<'r> FromFormField<'r> for B64b32 {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        use rocket::form::error::*;
        Ok(B64b32(LkHash::parse_str(field.value).map_err(|e| {
            ErrorKind::Validation(e.to_string().into())
        })?))
    }
    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        use rocket::data::ToByteUnit;
        use rocket::form::error::*;
        let st = field.data.open(43.bytes()).into_bytes().await?;
        Ok(B64b32(LkHash::try_fit_bytes_or_b64(&st).map_err(|e| {
            ErrorKind::Validation(e.to_string().into())
        })?))
    }
}
impl_from_uri_param_identity!(B64b32);
impl<'r> FromParam<'r> for B64b32 {
    type Error = base64::DecodeError;
    fn from_param<'a>(param: &'a str) -> std::result::Result<Self, Self::Error> {
        param.parse().map(B64b32)
    }
}

impl<P: uri::fmt::Part> UriDisplay<P> for B64b32 {
    fn fmt(&self, f: &mut uri::fmt::Formatter<P>) -> std::fmt::Result {
        f.write_value(self.0.b64())
    }
}

#[derive(FromForm)]
pub struct HashBody {
    pub hash: B64b32,
}

pub mod pkts_data {
    use std::sync::Arc;

    use linkspace::prelude::{ NetPktBox };

    pub struct Pkts<'o>(pub &'o Arc<[NetPktBox]>);
    use rocket::data::{self, Data, FromData, ToByteUnit};
    use rocket::http::Status;
    use rocket::request::Request;

    #[rocket::async_trait]
    impl<'r> FromData<'r> for Pkts<'r> {
        type Error = &'r anyhow::Error;

        async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
            use rocket::outcome::Outcome::*;
            let pkts = req.local_cache_async(async {
                // Use a configured limit with name 'person' or fallback to default.
                let limit = req.limits().get("pkts").unwrap_or(16.megabytes());
                let bytes = match data.open(limit).into_bytes().await {
                    Ok(b) if b.is_complete() => b.into_inner(),
                    Ok(_) => {
                        return Err((Status::PayloadTooLarge, anyhow::anyhow!("payload to large")))
                    }
                    Err(e) => return Err((Status::InternalServerError, e.into())),
                };
                let mut it = crate::utils::try_iter_pkts(bytes.as_slice());
                match it.try_collect::<Vec<_>>() {
                    Err(e) => return Err((Status::PreconditionFailed, e.into())),
                    Ok(e) => Ok(Arc::from(e)),
                }
            }).await;
            match pkts {
                Err((s,r)) => Failure((*s,r)),
                Ok(s) => Success(Pkts(s)),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LkPath<EXT>(pub IPathBuf, PhantomData<EXT>);
impl<A> LkPath<A> {
    pub fn any(self) -> AnyIPath {
        LkPath(self.0, PhantomData)
    }
    pub fn cast<B>(self) -> LkPath<B> {
        LkPath(self.0, PhantomData)
    }
}

impl AnyIPath {
    pub fn new(p: &IPath) -> AnyIPath {
        LkPath(p.to_owned(), PhantomData)
    }
}

pub type HtmlIPath = LkPath<HtmlExt>;
pub type AnyIPath = LkPath<()>;

pub trait IsExt {
    fn is_ext(segment: &[u8]) -> bool;
}

pub struct HtmlExt;
impl IsExt for HtmlExt {
    fn is_ext(segment: &[u8]) -> bool {
        segment.ends_with(b".html")
    }
}
impl IsExt for () {
    fn is_ext(_segment: &[u8]) -> bool {
        true
    }
}


pub fn ipath_uri_display(p: &SPath) -> Option<String> {
    p.iter()
        .map(|c| {
            Some(format!(
                "/{}",
                &std::str::from_utf8(c).ok()? as &dyn UriDisplay::<uri::fmt::Path>
            ))
        })
        .try_collect::<String>()
}

impl<EXT: IsExt> FromSegments<'_> for LkPath<EXT> {
    type Error = anyhow::Error;

    fn from_segments(segments: Segments<'_, Path>) -> anyhow::Result<Self> {
        let ip = IPathBuf::try_from_iter(segments.into_iter().map(|v| v.as_bytes()))?;
        anyhow::ensure!(EXT::is_ext(ip.last()), "wrong filetype");
        Ok(LkPath(ip, PhantomData))
    }
}
impl<E> UriDisplay<uri::fmt::Path> for LkPath<E> {
    fn fmt(&self, f: &mut uri::fmt::Formatter<uri::fmt::Path>) -> std::fmt::Result {
        for comp in self.0.iter() {
            let string = as_abtxt_c(comp, false);
            f.write_value(string)?;
        }
        Ok(())
    }
}
impl<E> FromUriParam<uri::fmt::Path, LkPath<E>> for LkPath<E> {
    type Target = Self;
    fn from_uri_param(param: LkPath<E>) -> Self::Target {
        param
    }
}

use rocket::{
    http::uri::{fmt::Path, Segments},
    request::FromSegments,
    *,
};

#[derive(Debug, PartialEq, FromFormField, Default)]
pub enum Editor {
    #[default]
    Default,
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
#[derive(Debug)]
pub struct Error(pub anyhow::Error);
impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Error(error.into())
    }
}
#[rocket::async_trait]
impl<'r> rocket::response::Responder<'r, 'static> for Error {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        (Status::InternalServerError, format!("{:#?}", self.0)).respond_to(req)
        //response::Debug(self.0).respond_to(req)
    }
}

pub static QUARANTINE: OnceLock<Vec<Host<'static>>> = OnceLock::new();
pub static WEBBIT: OnceLock<Vec<Host<'static>>> = OnceLock::new();


use rocket::outcome::try_outcome;
pub struct Quarantine;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Quarantine {
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let host = try_outcome!(request.host().or_forward(()));
        if host
            .to_absolute("http", &*QUARANTINE.get().unwrap())
            .is_some()
        {
            Outcome::Success(Quarantine)
        } else {
            Outcome::Forward(())
        }
    }
}

pub struct Webbit;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Webbit {
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let host = try_outcome!(request.host().or_forward(()));
        if host.to_absolute("http", &*WEBBIT.get().unwrap()).is_some() {
            Outcome::Success(Webbit)
        } else {
            Outcome::Forward(())
        }
    }
}

use rocket::request::Request;
use rocket::response::{self, Responder, Response};

pub struct HeaderHash<R>(pub LkHash, pub R);
impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for HeaderHash<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let header = Header::new("LK-Hash", self.0.to_string());
        Response::build()
            .merge(self.1.respond_to(req)?)
            .header(header)
            .ok()
    }
}

pub type ReqHeaderHash = HeaderHash<()>;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for ReqHeaderHash {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("LK-Hash") {
            Some(st) => match st.parse() {
                Ok(o) => Outcome::Success(HeaderHash(o, ())),
                Err(_) => Outcome::Failure((Status::BadRequest, ())),
            },
            None => Outcome::Forward(()),
        }
    }
}

#[derive(Clone)]
pub struct ReqQuery {
    pub query: Query,
    // Everything could be done with just query - they are separated here so we can redirect if the hash and path don't match
    pub hash: Option<LkHash>,
    pub path: AnyIPath,
    pub info: bool, // list | tree
}
impl ReqQuery {
    pub fn add_stmnt(self,key:&str,val:&str) -> anyhow::Result<Self> {
        let ReqQuery { mut query, mut hash, path, mut info } = self;
        match key {
            // whitelist query keys
            "alts" | "uploader" | "pkts" => {}
            "mode" => {
                query = lk_query_push(query, "","mode", val.as_bytes())?;
            },
            "follow" => {
                query = lk_query_push(query, "","follow", val.as_bytes())?;
            }
            "list" => {
                info = true;
                let depth = path.0.len() + if val.is_empty() { 0 } else { val.parse()? };
                let depth: u8 = depth.try_into()?;
                query = lk_query_push(query, "path_len", "=", &[depth])?;
            }
            "tree" => {
                info = true;
                eprintln!("val = {val} {}",val.is_empty());
                let depth = path.0.len() + if val.is_empty() { 255 } else { val.parse()? };
                let depth: u8 = depth.min(200).try_into()?;
                query = lk_query_push(query, "path_len", "<=", &[depth])?;
            }
            // TODO add :<: syntax
            e => {
                let predicate : PredicateType = e.parse().with_context(||e.to_string())?;

                if val.is_empty() {
                    if predicate == PredicateType::Pubkey{
                        query = lk_query_push(query, "type", "1",&[ PointTypeFlags::SIGNATURE.bits()])?;
                    }else {
                        bail!("missing value")
                    }
                } else {
                    if predicate == PredicateType::Hash{
                        hash = Some(val.parse()?)
                    }
                    let e = format!("{predicate}:=:{val}");
                    query = lk_query_parse(query, &[&e], ())?
                }
            },
        }
        Ok(ReqQuery { query, hash, path, info})
    }
}

pub struct LkQuery<'r>(pub &'r ReqQuery);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for LkQuery<'r> {
    type Error = String;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let r: &anyhow::Result<ReqQuery> = req.local_cache(|| {
            let uri = req.uri();
            let path: AnyIPath = req.segments(0..)?;
            let query = lk_query_push(lk_query(&crate::Q), "prefix", "=", path.0.spath_bytes())?;
            let mut rq = ReqQuery {
                info: false,
                hash: None,
                path,
                query,
            };
            if let Some(p) = uri.query() {
                for (key, val) in p.segments() {
                    rq = rq.add_stmnt(key, val).with_context(|| format!("{key} (={val})"))?;
                }
            }
            Ok(rq)
        });
        match r {
            Ok(q) => {
                tracing::debug!(query = lk_query_print(&q.query, false), "decoded query");
                outcome::Outcome::Success(LkQuery(q))
            }
            Err(e) => outcome::Outcome::Failure((Status::BadRequest, format!("{e:#?}"))),
        }
    }
}

/// A guard where either 'tree' or 'list' was used in the query param 
pub struct InfoQuery<'r>(pub &'r ReqQuery);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for InfoQuery<'r> {
    type Error = String;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, String> {
        let q = try_outcome!(request.guard::<LkQuery>().await);
        if q.0.info{
            Outcome::Success(InfoQuery(q.0))
        } else {
            Outcome::Forward(())
        }
    }
}

pub struct TailSlash;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for TailSlash {
    type Error = String;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, String> {
        let st = request.uri().path().as_str();
        tracing::info!(st,"request path str");
        if st.ends_with("/"){
            Outcome::Success(TailSlash)
        } else {
            Outcome::Forward(())
        }
    }
}
