use lazy_static::lazy_static;
use reqwest;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

lazy_static! {
    static ref API_URL: String =
        std::env::var("RV_API_URL").unwrap_or("http://localhost:4040/api".to_string());
    static ref RV_TERMINAL_SECRET: String =
        std::env::var("RV_TERMINAL_SECRET").unwrap_or("unsecure".to_string());
}

#[derive(Deserialize)]
pub struct AuthenticationResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
}

pub fn add_box(
    box_barcode: &str,
    product_barcode: &str,
    items_per_box: i32,
    credentials: &AuthenticationResponse,
) -> Result<ApiResult, reqwest::Error> {
    #[derive(Serialize)]
    struct Body {
        #[serde(rename = "boxBarcode")]
        box_barcode: String,
        #[serde(rename = "productBarcode")]
        product_barcode: String,
        #[serde(rename = "itemsPerBox")]
        items_per_box: i32,
    }
    let body: Body = Body {
        box_barcode: box_barcode.to_string(),
        items_per_box,
        product_barcode: product_barcode.to_string(),
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/admin/boxes", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&body)
        .send()
        .unwrap();
    match resp.status().as_u16() {
        201 => Ok(ApiResult::Success),
        409 => Ok(ApiResult::Fail(format!("error: barcode already in use"))),
        code => Ok(ApiResult::Fail(format!(
            "api request fail with code: {code}"
        ))),
    }
}

pub fn add_product(
    barcode: &str,
    name: &str,
    category_id: i32,
    buy_price: i32,
    sell_price: i32,
    stock: i32,
    credentials: &AuthenticationResponse,
) -> Result<ApiResult, reqwest::Error> {
    #[derive(Serialize)]
    struct Body {
        barcode: String,
        name: String,
        #[serde(rename = "categoryId")]
        category_id: i32,
        #[serde(rename = "buyPrice")]
        buy_price: i32,
        #[serde(rename = "sellPrice")]
        sell_price: i32,
        stock: i32,
    }
    let hm: Body = Body {
        barcode: barcode.to_string(),
        name: name.to_string(),
        category_id,
        buy_price,
        sell_price,
        stock,
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/admin/products/", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()?;
    match resp.status().as_u16() {
        201 => Ok(ApiResult::Success),
        409 => Ok(ApiResult::Fail(format!("error: barcode already in use"))),
        code => Ok(ApiResult::Fail(format!(
            "api request fail with code: {code}"
        ))),
    }
}

pub fn login(username: &str, password: &str) -> ApiResultValue<AuthenticationResponse> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/authenticate", *API_URL))
        .json(&HashMap::from([
            ("username", &username),
            ("password", &password),
            ("rvTerminalSecret", &RV_TERMINAL_SECRET.as_str()),
        ]))
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        200 => ApiResultValue::Success(
            resp.json::<AuthenticationResponse>()
                .expect("response json parse fail"),
        ),
        code => ApiResultValue::Fail(format!("api request fail with code: {code}")),
    }
}

pub fn login_rfid(rfid: &str) -> Option<AuthenticationResponse> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/authenticate/rfid", *API_URL))
        .json(&HashMap::from([
            ("rfid", &rfid),
            ("rvTerminalSecret", &RV_TERMINAL_SECRET.as_str()),
        ]))
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        200 => Some(
            resp.json::<AuthenticationResponse>()
                .expect("response json parse fail"),
        ),
        _ => None,
    }
}

#[derive(Deserialize, Debug)]
pub struct UserInfo {
    #[serde(rename = "userId")]
    pub user_id: i32,
    pub username: String,
    //#[serde(rename = "fullName")] note this is optionaö!!
    //pub full_name: String,
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "moneyBalance")]
    pub money_balance: i32,
    pub role: String,
}

pub trait UserInfoTrait {
    fn is_admin(&self) -> bool;
}

impl UserInfoTrait for UserInfo {
    fn is_admin(&self) -> bool {
        self.role == "ADMIN"
    }
}

pub fn get_user_info(credentials: &AuthenticationResponse) -> Result<UserInfo, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        user: UserInfo,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/v1/user", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("api error");
    return resp.json::<Hax>().map(|v| v.user);
}

pub fn get_user_info_by_username(
    credentials: &AuthenticationResponse,
    username: &str,
) -> Result<ApiResultValue<UserInfo>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        user: UserInfo,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!(
            "{}/v1/admin/utils/getUserByUsername/{username}",
            *API_URL
        ))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("api error");
    Ok(match resp.status().as_u16() {
        200 => ApiResultValue::Success(resp.json::<Hax>().map(|v| v.user).unwrap()),
        404 => ApiResultValue::Fail("User with the given username not found".to_string()),
        401 => ApiResultValue::Fail("Not authorized".to_string()),
        code => ApiResultValue::Fail(format!("http response {code}")),
    })
}

pub enum ApiResultValue<T> {
    Success(T),
    Fail(String),
}
pub enum ApiResult {
    Success,
    Fail(String),
}

pub fn buy_in_box(
    barcode: &str,
    product_buy_price: i32,
    product_sell_price: i32,
    box_count: i32,
    credentials: &AuthenticationResponse,
) -> Result<ApiResult, reqwest::Error> {
    #[derive(Serialize)]
    struct Body {
        #[serde(rename = "boxCount")]
        box_count: i32,
        #[serde(rename = "productBuyPrice")]
        product_buy_price: i32,
        #[serde(rename = "productSellPrice")]
        product_sell_price: i32,
    }
    let hm: Body = Body {
        box_count,
        product_buy_price,
        product_sell_price,
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/admin/boxes/{barcode}/buyIn", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()
        .unwrap();
    match resp.status().as_u16() {
        200 => Ok(ApiResult::Success),
        code => Ok(ApiResult::Fail(format!(
            "api request fail with code: {code}"
        ))),
    }
}

pub fn change_password_admin(
    credentials: &AuthenticationResponse,
    user_id: i32,
    password: &str,
) -> Result<ApiResult, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!(
            "{}/v1/admin/users/{user_id}/changePassword",
            *API_URL
        ))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&HashMap::from([("password", password)]))
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        200 => Ok(ApiResult::Success),
        404 => Ok(ApiResult::Fail(
            "User with the given username not found".to_string(),
        )),
        400 => Ok(ApiResult::Fail(
            "Missing or invalid fields in request".to_string(),
        )),
        401 => Ok(ApiResult::Fail("Not authorized".to_string())),
        code => Ok(ApiResult::Fail(format!("http response {code}"))),
    }
}
pub fn change_password(
    credentials: &AuthenticationResponse,
    password: &str,
) -> Result<ApiResult, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/user/changePassword", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&HashMap::from([("password", password)]))
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        204 => Ok(ApiResult::Success),
        400 => Ok(ApiResult::Fail(
            "Missing or invalid fields in request".to_string(),
        )),
        401 => Ok(ApiResult::Fail("Not authorized".to_string())),
        code => Ok(ApiResult::Fail(format!("http response {code}"))),
    }
}

pub fn change_rfid(
    credentials: &AuthenticationResponse,
    rfid: &str,
) -> Result<ApiResult, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/user/changeRfid", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&HashMap::from([("rfid", rfid)]))
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        204 => Ok(ApiResult::Success),
        400 => Ok(ApiResult::Fail(
            "Missing or invalid fields in request".to_string(),
        )),
        401 => Ok(ApiResult::Fail("Not authorized".to_string())),
        code => Ok(ApiResult::Fail(format!("http response {code}"))),
    }
}

pub fn return_product(
    credentials: &AuthenticationResponse,
    barcode: &str,
) -> Result<ApiResult, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/products/{barcode}/return", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("api error");

    match resp.status().as_u16() {
        200 => Ok(ApiResult::Success),
        403 => Ok(ApiResult::Fail({
            #[derive(Deserialize)]
            struct Hax {
                message: String,
            }
            resp.json::<Hax>()
                .map(|v| v.message)
                .unwrap_or("unknown 403 error".to_string())
        })),
        code => Ok(ApiResult::Fail(format!("http response {code}"))),
    }
}

pub fn purchase_item(
    credentials: &AuthenticationResponse,
    barcode: &str,
    count: &i32,
) -> Result<ApiResult, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/products/{barcode}/purchase", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&HashMap::from([("count", count)]))
        .send()
        .expect("api error");

    match resp.status().as_u16() {
        200 => Ok(ApiResult::Success),
        404 => Ok(ApiResult::Fail(format!(
            "No product with barcode {barcode} found!"
        ))),
        403 => Ok(ApiResult::Fail({
            #[derive(Deserialize)]
            struct Hax {
                message: String,
            }
            resp.json::<Hax>()
                .map(|v| v.message)
                .unwrap_or("unknown 403 error".to_string())
        })),
        code => Ok(ApiResult::Fail(format!("http response {code}"))),
    }
}

pub fn deposit(
    credentials: &AuthenticationResponse,
    amount: &u32,
    deposit_type: &str,
) -> Result<(), reqwest::Error> {
    #[derive(Serialize)]
    struct Body {
        amount: u32,
        #[serde(rename = "type")]
        deposit_type: String,
    }
    let client = reqwest::blocking::Client::new();
    client
        .post(format!("{}/v1/user/deposit", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&Body {
            amount: *amount,
            deposit_type: deposit_type.to_string(),
        })
        .send()?;
    Ok(())
}

#[derive(Deserialize)]
pub struct ProductCategory {
    #[serde(rename = "categoryId")]
    pub category_id: i32,
    pub description: String,
}

#[derive(Deserialize)]
pub struct ProductInfoAdmin {
    pub barcode: String,
    pub name: String,
    #[serde(rename = "sellPrice")]
    pub sell_price: i32,
    #[serde(rename = "buyPrice")]
    pub buy_price: i32,
    pub category: ProductCategory,
    pub stock: i32,
}

#[derive(Deserialize)]
pub struct BoxInfoAdmin {
    #[serde(rename = "boxBarcode")]
    pub box_barcode: String,
    #[serde(rename = "itemsPerBox")]
    pub items_per_box: i32,
    pub product: ProductInfoAdmin,
}

pub fn get_box_info_admin(
    barcode: &str,
    credentials: &AuthenticationResponse,
) -> Result<Option<BoxInfoAdmin>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        #[serde(rename = "box")]
        box_: BoxInfoAdmin,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/v1/admin/boxes/{barcode}", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("");
    match resp.status().as_u16() {
        200 => Ok(Some(resp.json::<Hax>().map(|v| v.box_).unwrap())),
        404 => Ok(None),
        code => panic!("{}", (format!("error code: {}", code))),
    }
}

pub fn get_product_info_admin(
    credentials: &AuthenticationResponse,
    barcode: &str,
) -> Result<ApiResultValue<ProductInfoAdmin>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        product: ProductInfoAdmin,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/v1/admin/products/{barcode}", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        200 => Ok(ApiResultValue::Success(
            resp.json::<Hax>().map(|v| v.product)?,
        )),
        404 => Ok(ApiResultValue::Fail("Product not found".to_string())),
        code => Ok(ApiResultValue::Fail(format!("error code: {}", code))),
    }
}

#[derive(Deserialize)]
pub struct ProductInfo {
    pub barcode: String,
    pub name: String,
    #[serde(rename = "sellPrice")]
    pub price: i32,
    pub stock: i32,
}

pub fn buy_in_product(
    barcode: &str,
    buy_price: i32,
    sell_price: i32,
    count: i32,
    credentials: &AuthenticationResponse,
) -> ApiResult {
    #[derive(Serialize)]
    struct Body {
        #[serde(rename = "buyPrice")]
        buy_price: i32,
        #[serde(rename = "sellPrice")]
        sell_price: i32,
        count: i32,
    }
    let hm: Body = Body {
        buy_price,
        sell_price,
        count,
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/admin/products/{barcode}/buyIn", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()
        .unwrap();
    match resp.status().as_u16() {
        200 => ApiResult::Success,
        code => ApiResult::Fail(format!("api request fail with code: {code}")),
    }
}

pub fn update_box(
    barcode: &str,
    items_per_box: i32,
    product_barcode: &str,
    credentials: &AuthenticationResponse,
) -> ApiResult {
    #[derive(Serialize)]
    struct Body {
        #[serde(rename = "itemsPerBox")]
        items_per_box: i32,
        #[serde(rename = "productBarcode")]
        product_barcode: String,
    }
    let hm: Body = Body {
        items_per_box,
        product_barcode: product_barcode.to_string(),
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .patch(format!("{}/v1/admin/boxes/{barcode}", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()
        .unwrap();
    match resp.status().as_u16() {
        200 => ApiResult::Success,
        code => ApiResult::Fail(format!("api request fail with code: {code}")),
    }
}

pub fn update_product(
    barcode: &str,
    name: &str,
    category_id: i32,
    buy_price: i32,
    sell_price: i32,
    stock: i32,
    credentials: &AuthenticationResponse,
) -> Result<(), reqwest::Error> {
    #[derive(Serialize)]
    struct Body {
        name: String,
        #[serde(rename = "categoryId")]
        category_id: i32,
        #[serde(rename = "buyPrice")]
        buy_price: i32,
        #[serde(rename = "sellPrice")]
        sell_price: i32,
        stock: i32,
    }
    let hm: Body = Body {
        name: name.to_string(),
        category_id,
        buy_price,
        sell_price,
        stock,
    };
    let client = reqwest::blocking::Client::new();
    let resp = client
        .patch(format!("{}/v1/admin/products/{barcode}", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()?;
    match resp.status().as_u16() {
        200 => Ok(()),
        _ => panic!(),
    }
}

pub fn search_boxes(
    credentials: &AuthenticationResponse,
    query: &str,
) -> Result<Vec<BoxInfoAdmin>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        boxes: Vec<BoxInfoAdmin>,
    }
    let client = reqwest::blocking::Client::new();
    let hm = HashMap::from([("query", &query)]);
    let resp = client
        .post(format!("{}/v1/admin/boxes/search", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()
        .expect("api error");
    return resp.json::<Hax>().map(|v| v.boxes);
}

pub fn search_products(
    credentials: &AuthenticationResponse,
    query: &str,
) -> Result<Vec<ProductInfo>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        products: Vec<ProductInfo>,
    }
    let client = reqwest::blocking::Client::new();
    let hm = HashMap::from([("query", &query)]);
    let resp = client
        .post(format!("{}/v1/products/search", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&hm)
        .send()
        .expect("api error");
    return resp.json::<Hax>().map(|v| v.products);
}

pub fn get_product_info(
    credentials: &AuthenticationResponse,
    barcode: &str,
) -> Option<ProductInfo> {
    #[derive(Deserialize)]
    struct Hax {
        product: ProductInfo,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/v1/products/{barcode}", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()
        .expect("api error");
    match resp.status().as_u16() {
        200 => return Some(resp.json::<Hax>().map(|v| v.product).unwrap()),
        404 => return None,
        code => {
            panic!("api error: {}", format!("{code}"));
        }
    }
}

pub fn user_exists(username: &str) -> Result<bool, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        exists: bool,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/user/user_exists", *API_URL))
        .json(&HashMap::from([("username", &username)]))
        .send()
        .expect("api error");
    Ok(resp.json::<Hax>().unwrap().exists)
}

pub fn register(
    username: &str,
    password: &str,
    full_name: &str,
    email: &str,
) -> Result<ApiResult, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        #[serde(default)]
        message: String,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/v1/register", *API_URL))
        .json(&HashMap::from([
            ("username", &username),
            ("password", &password),
            ("email", &email),
            ("fullName", &full_name),
        ]))
        .send()
        .expect("api error");
    //Ok()
    match resp.status().as_u16() {
        201 => Ok(ApiResult::Success),
        409 => Ok(ApiResult::Fail(resp.json::<Hax>().unwrap().message)),
        code => Ok(ApiResult::Fail(format!("error code {code}"))),
    }
}

//credentials: &AuthenticationResponse,
pub fn set_margin(margin: f32, credentials: &AuthenticationResponse) -> Result<(), reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .patch(format!(
            "{}/v1/admin/preferences/globalDefaultMargin",
            *API_URL
        ))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .json(&HashMap::from([("value", margin)]))
        .send()?;
    if resp.status().as_u16() != 200 {
        panic!();
    }
    Ok(())
}

pub fn get_margin(credentials: &AuthenticationResponse) -> Result<f32, reqwest::Error> {
    #[derive(Deserialize)]
    struct Preference {
        key: String,
        value: f32,
    }
    #[derive(Deserialize)]
    struct Hax {
        preference: Preference,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!(
            "{}/v1/admin/preferences/globalDefaultMargin",
            *API_URL
        ))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()?;
    Ok(resp.json::<Hax>().unwrap().preference.value)
}

pub fn get_categories(
    credentials: &AuthenticationResponse,
) -> Result<Vec<ProductCategory>, reqwest::Error> {
    #[derive(Deserialize)]
    struct Hax {
        categories: Vec<ProductCategory>,
    }
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/v1/categories", *API_URL))
        .header(
            "Authorization",
            String::from("Bearer ") + &credentials.access_token,
        )
        .send()?;
    Ok(resp.json::<Hax>().unwrap().categories)
}
