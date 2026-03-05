use redis::TypedCommands;
use crate::errors::ServerError;

pub async fn get_df_payments(
    redis_cli : redis::Client
) -> Result<Option<isize>,ServerError>{
    let mut con = redis_cli.get_connection()?;
    Ok(con.get_int("default_payments")?)
}

pub async fn get_fb_payments(
    redis_cli : redis::Client
) -> Result<Option<isize>,ServerError>{
    let mut con = redis_cli.get_connection()?;
    Ok(con.get_int("fallback_payments")?)
}

pub async fn inc_df_payments(
    redis_cli : &redis::Client
) -> Result<(),ServerError>{
    let mut con = redis_cli.get_connection()?;
    con.incr("default_payments",1)?;
    Ok(())
}

pub async fn inc_fb_payments(
    redis_cli : &redis::Client
) -> Result<(),ServerError>{
    let mut con = redis_cli.get_connection()?;
    con.incr("fallback_payments",1)?;
    Ok(())
}