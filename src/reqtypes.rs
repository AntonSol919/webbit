use std::{ marker::PhantomData, sync::{ OnceLock} };
use rocket::{http::{uri::{fmt::{FromUriParam, UriDisplay}, self, Host}, impl_from_uri_param_identity, Status, Header }, form::{FromFormField, ValueField, DataField}, request::{FromRequest, Outcome}, outcome::IntoOutcome };
use linkspace::prelude::{* };


#[derive(Debug,Copy,Clone,PartialEq,Eq)]
pub struct Hash(pub B64<[u8;32]>);
#[rocket::async_trait]
impl<'r> FromFormField<'r> for Hash {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        use rocket::form::error::*;
        Ok(Hash(LkHash::parse_str(field.value).map_err(|e| ErrorKind::Validation(e.to_string().into()))?))
    }
    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        use rocket::form::error::*;
        use rocket::data::ToByteUnit;
        let st = field.data.open(67.bytes()).into_bytes().await?;
        Ok(Hash(LkHash::try_fit_bytes_or_b64(&st).map_err(|e| ErrorKind::Validation(e.to_string().into()))?))
    }
}
impl_from_uri_param_identity!(Hash);

impl<P:uri::fmt::Part> UriDisplay<P> for Hash{
    fn fmt(&self, f: &mut uri::fmt::Formatter<P>) -> std::fmt::Result {
        f.write_value(self.0.b64())
    }
}

#[derive(FromForm)]
pub struct HashBody {
    pub hash: Hash,
}


pub mod pkts_data{
    use linkspace::prelude::{ NetPktBox, PubKey };

    pub struct Pkts<'o>(pub &'o [NetPktBox]);
    use rocket::request::{ Request };
    use rocket::data::{self, Data, FromData, ToByteUnit};
    use rocket::http::{Status };
    pub struct SignedBy(pub Option<PubKey>);
    #[rocket::async_trait]
    impl<'r> FromData<'r> for Pkts<'r>{
        type Error = anyhow::Error;

        async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
            use rocket::outcome::Outcome::*;

            // Use a configured limit with name 'person' or fallback to default.
            let limit = req.limits().get("pkts").unwrap_or(16.megabytes());
            
            let bytes = match data.open(limit).into_bytes().await{
                Ok(b) if b.is_complete() => b.into_inner(),
                Ok(_) => return Failure((Status::PayloadTooLarge, anyhow::anyhow!("payload to large"))),
                Err(e) => return Failure((Status::InternalServerError, e.into())),
           };
            let mut it = crate::utils::try_iter_pkts(bytes.as_slice());
            let result = match it.try_collect::<Vec<_>>(){
                Err(e) => return Failure((Status::PreconditionFailed,e.into())),
                Ok(e) => e
            };
            let pkts= req.local_cache(|| result);

            Success(Pkts(pkts.as_slice()))
        }
    }
}

#[derive(Debug,Clone)]
pub struct LkPath<EXT>(pub IPathBuf,PhantomData<EXT>);
impl<A> LkPath<A>{
    pub fn any(self) -> AnyIPath{LkPath(self.0,PhantomData)}
    pub fn cast<B>(self) -> LkPath<B>{ LkPath(self.0,PhantomData)}
}

impl AnyIPath { pub fn new(p:&IPath) -> AnyIPath{ LkPath(p.to_owned(),PhantomData)}}

pub type HtmlIPath = LkPath<HtmlExt>;
pub type AnyIPath = LkPath<()>;

pub trait IsExt { fn is_ext(segment: &[u8]) -> bool;}

pub struct HtmlExt;
impl IsExt for HtmlExt { fn is_ext(segment:&[u8]) -> bool {segment.ends_with(b".html")}}
impl IsExt for (){ fn is_ext(_segment:&[u8]) -> bool {true}}

pub fn ipath_uri_display(p:&SPath) -> Option<String>{
    p.iter().map(|c|Some(format!("/{}",& std::str::from_utf8(c).ok()? as &dyn UriDisplay::<uri::fmt::Path>))).try_collect::<String>()
}

impl<EXT : IsExt> FromSegments<'_> for LkPath<EXT>{
    type Error = anyhow::Error;

    fn from_segments(segments: Segments<'_, Path>) -> anyhow::Result<Self> {
        let ip = IPathBuf::try_from_iter(segments.into_iter().map(|v| v.as_bytes()))?;
        anyhow::ensure!(EXT::is_ext(ip.last()),"wrong filetype");
        Ok(LkPath(ip,PhantomData))
    }
}
impl<E> UriDisplay<uri::fmt::Path> for LkPath<E>{
    fn fmt(&self, f: &mut uri::fmt::Formatter<uri::fmt::Path>) -> std::fmt::Result {
        for comp in self.0.iter(){
            let string = as_abtxt_c(comp, false);
            f.write_value(string)?;
        }
        Ok(())
     }
}
impl<E> FromUriParam<uri::fmt::Path,LkPath<E>> for LkPath<E> {
    type Target = Self;
    fn from_uri_param(param: LkPath<E>) -> Self::Target {param}
}

use rocket::{*, request::{ FromSegments}, http::uri::{fmt::Path, Segments}};

#[derive(Debug, PartialEq, FromFormField,Default)]
pub enum Editor{
    #[default]
    Default,
}

pub type Result<T = (),E = Error> = std::result::Result<T, E>;
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
impl<'r> rocket::response::Responder<'r, 'static> for Error{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        (Status::InternalServerError,format!("{:#?}",self.0)).respond_to(req)
        //response::Debug(self.0).respond_to(req)
    }
}


pub static QUARANTINE: OnceLock<Vec<Host<'static>>> = OnceLock::new();
pub static WEBBIT : OnceLock<Vec<Host<'static>>> = OnceLock::new();

use rocket::outcome::try_outcome;
pub struct Quarantine;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Quarantine{
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let host = try_outcome!(request.host().or_forward(()));
        if host.to_absolute("http",&*QUARANTINE.get().unwrap()).is_some(){
            Outcome::Success(Quarantine)
        }else {
            Outcome::Forward(())
        }
    }
}

pub struct Webbit;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Webbit{
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let host = try_outcome!(request.host().or_forward(()));
        if host.to_absolute("http",&*WEBBIT.get().unwrap()).is_some(){
            Outcome::Success(Webbit)
        }else {
            Outcome::Forward(())
        }
    }
}




use rocket::request::Request;
use rocket::response::{self, Response, Responder};

pub struct HeaderHash<R>(pub LkHash,pub R);
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
impl<'r> FromRequest<'r> for ReqHeaderHash{
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("LK-Hash"){
            Some(st) => match st.parse(){
                Ok(o) => Outcome::Success(HeaderHash(o,())),
                Err(_) => Outcome::Failure((Status::BadRequest,()))
            },
            None => Outcome::Forward(()),
        }
        
    }
}
