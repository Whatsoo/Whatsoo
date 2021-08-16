use crate::common::api::ApiResult;
use crate::common::constant::TOKEN_HEADER_NAME;
use crate::common::util;
use crate::model::user::{CaptchaUser, LoginUser, RegisterUser, User, UserToken, VerifyStatus};
use crate::service::user_service;
use crate::AppState;
use crate::MAILE_RE;
use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::Local;
use r2d2_redis::r2d2::PooledConnection;
use r2d2_redis::redis::RedisError;
use r2d2_redis::RedisConnectionManager;

#[get("/user/validate/email/{email}")]
async fn validate_email(web::Path(email): web::Path<String>, state: AppState) -> impl Responder {
    let legal = MAILE_RE.is_match(&email);
    if !legal {
        return ApiResult::error()
            .data(VerifyStatus::fail())
            .msg("邮箱格式不合法，请重新出入邮箱");
    } else {
        user_service::check_email_exists(email, &state.get_ref().db_pool).await
    }
}

#[get("/user/validate/username/{username}")]
async fn validate_username(username: web::Path<String>, state: AppState) -> impl Responder {
    user_service::check_username_exists(username.into_inner(), &state.get_ref().db_pool).await
}

#[get("/captcha")]
async fn get_captcha(state: AppState) -> impl Responder {
    let (key, vec) = util::gen_pic_captcha(&mut state.get_ref().redis_pool.get().unwrap()).await;
    HttpResponse::Ok().header("captcha-key", key).body(vec)
}

#[get("/verify/captcha")]
async fn verify_captcha(captcha_user: web::Form<CaptchaUser>, state: AppState) -> impl Responder {
    let connection = &mut state.get_ref().redis_pool.get().unwrap();
    let is_valid = util::validate_captcha(
        &captcha_user.captcha_key,
        &captcha_user.captcha_value,
        connection,
    )
    .await;
    if is_valid {
        let legal = MAILE_RE.is_match(&captcha_user.email);
        if legal {
            let email_verify_code = util::send_email(&captcha_user.email).await;
            util::redis_set(&captcha_user.email, &email_verify_code, 60 * 50, connection).await;
            return ApiResult::ok()
                .msg("验证码校验成功，已发送验证码到您邮箱，请查收")
                .data(VerifyStatus::success());
        } else {
            return ApiResult::ok()
                .msg("验证码校验失败, 邮箱格式不合法")
                .data(VerifyStatus::fail());
        }
    } else {
        ApiResult::ok()
            .msg("验证码校验失败")
            .data(VerifyStatus::fail())
    }
}

#[get("/verify/email")]
async fn verify_email(register_user: web::Form<RegisterUser>, state: AppState) -> impl Responder {
    let connection = &mut state.get_ref().redis_pool.get().unwrap();
    let pool = &state.get_ref().db_pool;
    user_service::register_user(register_user.into_inner(), connection, pool).await
}

#[get("/login")]
async fn login(login_user: web::Form<LoginUser>, state: AppState) -> impl Responder {
    let connection = &mut state.get_ref().redis_pool.get().unwrap();
    let pool = &state.get_ref().db_pool;
    let is_valid = util::validate_captcha(
        &login_user.captcha_key,
        &login_user.captcha_value,
        connection,
    )
    .await;
    // 验证码不正确直接返回
    if !is_valid {
        return HttpResponse::Ok().json(
            ApiResult::ok()
                .msg("验证码校验失败")
                .data(VerifyStatus::fail()),
        );
    }
    let user = User::find_user_by_email(&login_user.email, pool).await;
    if let Some(u) = user {
        let login_success = util::verify_pwd(&login_user.password, &u.user_password).await;
        if login_success {
            let exp: usize = if login_user.forever {
                0 as usize
            } else {
                (Local::now().timestamp() + 60 * 60 * 24 * 30) as usize
            };
            let user_token =
                util::token_encode(&UserToken::new(u.pk_id, u.uk_username, u.uk_email, exp)).await;
            return HttpResponse::Ok()
                .header(TOKEN_HEADER_NAME, user_token)
                .json(
                    ApiResult::ok()
                        .msg("登录成功")
                        .data(VerifyStatus::success()),
                );
        } else {
            return HttpResponse::Ok().json(
                ApiResult::error()
                    .msg("登录失败，用户名或密码错误")
                    .data(VerifyStatus::fail()),
            );
        }
    } else {
        return HttpResponse::Ok().json(
            ApiResult::error()
                .msg("用户不存在")
                .data(VerifyStatus::fail()),
        );
    }
}

// function that will be called on new Application to configure routes for this module
#[inline]
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(verify_email)
        .service(validate_email)
        .service(validate_username)
        .service(get_captcha)
        .service(verify_captcha)
        .service(login);
}
