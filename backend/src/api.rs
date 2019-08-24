use actix_web::HttpResponse;
pub use index3ds_common::*;

pub trait ToHttpResponse {
    fn http(&self) -> HttpResponse;
}

impl ToHttpResponse for NcchInfoResponse {
    fn http(&self) -> HttpResponse {
        match self {
            NcchInfoResponse::Ok(_) => HttpResponse::Ok(),
            NcchInfoResponse::NotFound => HttpResponse::NotFound(),
            NcchInfoResponse::InternalServerError => HttpResponse::InternalServerError(),
        }
        .json(self)
    }
}

impl ToHttpResponse for PostNcchResponse {
    fn http(&self) -> HttpResponse {
        match self {
            PostNcchResponse::Finished(_) | PostNcchResponse::AppendNeeded(_) => HttpResponse::Ok(),
            PostNcchResponse::AlreadyFinished
            | PostNcchResponse::UnexpectedLength
            | PostNcchResponse::UnexpectedFormat
            | PostNcchResponse::VerificationFailed => HttpResponse::BadRequest(),
            PostNcchResponse::Busy => HttpResponse::ServiceUnavailable(),
            PostNcchResponse::Conflict(_) => HttpResponse::Conflict(),
            PostNcchResponse::InternalServerError => HttpResponse::InternalServerError(),
            PostNcchResponse::NotFound => HttpResponse::NotFound(),
        }
        .json(self)
    }
}

impl ToHttpResponse for NcchQueryResponse {
    fn http(&self) -> HttpResponse {
        match self {
            NcchQueryResponse::Ok(_) => HttpResponse::Ok(),
            NcchQueryResponse::InternalServerError => HttpResponse::InternalServerError(),
        }
        .json(self)
    }
}

impl ToHttpResponse for NcchQueryCountResponse {
    fn http(&self) -> HttpResponse {
        match self {
            NcchQueryCountResponse::Ok(_) => HttpResponse::Ok(),
            NcchQueryCountResponse::InternalServerError => HttpResponse::InternalServerError(),
        }
        .json(self)
    }
}
