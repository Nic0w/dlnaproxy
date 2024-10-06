use std::net::ToSocketAddrs;

use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};

pub struct WebProxy;

