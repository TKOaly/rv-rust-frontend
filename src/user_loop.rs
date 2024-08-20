use super::input;
use super::rv_api;
use super::utils;
use crate::rv_api::get_box_info_admin;
use crate::rv_api::get_product_info;
use crate::rv_api::get_user_info;
use crate::rv_api::return_product;
use crate::rv_api::update_box;
use crate::rv_api::ApiResultValue;
use crate::rv_api::ProductCategory;
use crate::rv_api::UserInfoTrait;
use crate::utils::clear_terminal;
use crate::utils::print_error_line;
use crate::utils::print_title;
use crate::utils::printline;
use crate::utils::readline;
use crate::utils::TimeoutResult;
use crate::TerminalIO;
use crate::INPUT_TIMEOUT_LONG;
use crate::INPUT_TIMEOUT_SHORT;

use crossterm::queue;
use crossterm::style::Color;
use crossterm::style::PrintStyledContent;
use crossterm::style::Stylize;
use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute,
    style::Print,
    terminal::{self, disable_raw_mode},
    ExecutableCommand,
};
use input::InputEvent;
use regex::Regex;
use rv_api::ApiResult;
use std::process::exit;
use std::sync::mpsc::RecvTimeoutError;
static RV_LOGO: &str = " \
______     __\r\n|  _ \\ \\   / /\r\n| |_) \\ \\ / / \r\n|  _ < \\ V /  \r\n|_| \\_\\ \\_/   \r\n\
\r\n\
";

fn new_product(
    barcode: &str,
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    let name;
    utils::printline(
        terminal_io,
        &format!("Creating a new product. Enter to cancel."),
    );
    utils::printline(terminal_io, &format!("Enter product name:"));

    let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };
    if input_line.len() > 0 {
        name = input_line.to_string();
    } else {
        printline(terminal_io, "Cancelled.");
        return TimeoutResult::RESULT(());
    }

    printline(terminal_io, "");

    let buy_price = loop {
        utils::printline(
            terminal_io,
            "Enter item buyprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Cancelled.");
            return TimeoutResult::RESULT(());
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            break input_line.replace(".", "").parse().unwrap();
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    };
    printline(terminal_io, "");

    let sell_price = loop {
        utils::printline(
            terminal_io,
            "\r\nEnter item sellprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
        let margin = rv_api::get_margin(&credentials).unwrap();
        let margin_pretty = format!("{}%", (margin * 100.0).ceil());
        let suggested_price = (buy_price as f32 * (1.0 + margin)).ceil() as i32;
        utils::printline(
            terminal_io,
            &format!(
                "Suggest {} calculated with the margin of {}",
                &utils::format_money(&suggested_price),
                margin_pretty
            ),
        );
        utils::printline(
            terminal_io,
            &format!(
                "Modify or keep [{}]:",
                &utils::format_money(&suggested_price)
            ),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Using the suggested price.");
            break suggested_price;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            break input_line.replace(".", "").parse().unwrap();
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    };
    printline(terminal_io, "");
    let stock = loop {
        let suggested_stock = 0;
        utils::printline(terminal_io, "Enter item stock. Format: [0-9]+");
        utils::printline(terminal_io, &format!("Modify or keep [{suggested_stock}]"));
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break suggested_stock;
        } else if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            break input_line.parse().unwrap();
        } else {
            print_error_line(terminal_io, "Invalid stock entered, please retry!\n");
        }
    };
    printline(terminal_io, "");

    let category = loop {
        utils::printline(terminal_io, "Enter product category id.");
        utils::printline(terminal_io, "Categories available:");
        let categories = rv_api::get_categories(&credentials).unwrap();
        for category in categories.iter() {
            utils::printline(
                terminal_io,
                &format!("{}, id: {}", category.description, category.category_id),
            );
        }
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            print_error_line(terminal_io, "Invalid category id entered, please retry!\n");
            continue;
        } else if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            let chosen: i32 = input_line.parse().unwrap();
            match categories.iter().find(|c| c.category_id == chosen) {
                Some(c) => {
                    break ProductCategory {
                        description: c.description.clone(),
                        category_id: c.category_id,
                    };
                }
                None => {
                    print_error_line(terminal_io, "Invalid category id entered, please retry!\n");
                    continue;
                }
            }
        } else {
            print_error_line(terminal_io, "Invalid category entered, please retry!\n");
        }
    };
    match rv_api::add_product(
        barcode,
        &name,
        category.category_id,
        buy_price,
        sell_price,
        stock,
        credentials,
    )
    .unwrap()
    {
        ApiResult::Success => utils::printline(terminal_io, "Product added."),
        ApiResult::Fail(msg) => utils::print_error_line(terminal_io, &msg),
    }
    TimeoutResult::RESULT(())
}

fn new_box(
    barcode: &str,
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    utils::printline(terminal_io, "Creating a new box.");
    let product_barcode = loop {
        utils::printline(terminal_io, "Enter product barcode.");
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            utils::printline(terminal_io, "Cancelled.");
            return TimeoutResult::RESULT(());
        } else if Regex::new("^[0-9]+$").expect("").is_match(&input_line) {
            let product_barcode = input_line;
            match rv_api::get_product_info(credentials, &product_barcode) {
                Some(product_info) => {
                    utils::printline(
                        terminal_io,
                        &format!(
                            "Found an existing product with the given barcode: {}",
                            product_info.name
                        ),
                    );
                }
                None => {
                    utils::printline(
                        terminal_io,
                        "Couldn't find an existing product with the given barcode.",
                    );
                    utils::printline(terminal_io, "");
                    if let TimeoutResult::TIMEOUT =
                        new_product(&product_barcode, credentials, terminal_io)
                    {
                        return TimeoutResult::TIMEOUT;
                    }
                    if get_product_info(credentials, &product_barcode).is_none() {
                        print_error_line(terminal_io, "Adding new product failed!");
                        return TimeoutResult::RESULT(());
                    }
                }
            }
            break product_barcode;
        } else {
            print_error_line(terminal_io, "Invalid barcode entered, please retry!\n");
        }
    };
    printline(terminal_io, "");

    let items_per_box;
    loop {
        utils::printline(
            terminal_io,
            "Enter number of products in a box. Format: [0-9]+",
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            items_per_box = input_line.parse().unwrap();
            break;
        } else {
            print_error_line(terminal_io, "Invalid number entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    match rv_api::add_box(barcode, &product_barcode, items_per_box, credentials).unwrap() {
        ApiResult::Success => {
            utils::printline(terminal_io, &format!("Box added."));
            utils::printline(terminal_io, &format!(""));
            return buy_in_box(barcode, credentials, terminal_io);
        }
        ApiResult::Fail(msg) => print_error_line(terminal_io, &msg),
    }
    TimeoutResult::RESULT(())
}

fn new_item(
    barcode: &str,
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    printline(
        terminal_io,
        &format!("Add a new box or product? [bp] or Enter to cancel."),
    );
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_LONG) {
            Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
            Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                KeyCode::Enter => {
                    utils::printline(terminal_io, "Cancelled!");
                    return TimeoutResult::RESULT(());
                }
                KeyCode::Char(c) => match c {
                    'b' | 'B' => {
                        utils::printline(terminal_io, "");
                        return new_box(barcode, credentials, terminal_io);
                    }
                    'p' | 'P' => {
                        utils::printline(terminal_io, "");
                        return new_product(barcode, credentials, terminal_io);
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
    }
}

fn buy_in_box(
    barcode: &str,
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    let box_ = match rv_api::get_box_info_admin(barcode, credentials).unwrap() {
        Some(b) => b,
        None => {
            print_error_line(
                terminal_io,
                &format!("Buy in error: No box found with barcode {}", barcode),
            );
            return TimeoutResult::RESULT(());
        }
    };
    utils::printline(
        terminal_io,
        &format!(
            "Found a box containing {}x of {}",
            box_.items_per_box, box_.product.name
        ),
    );

    utils::printline(terminal_io, "Adding new box to stock.");
    let mut buy_price = box_.product.buy_price;
    let mut buy_price_changed = false;
    loop {
        utils::printline(
            terminal_io,
            "Enter box buyprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
        utils::printline(
            terminal_io,
            &format!(
                "Modify or keep [{}]:",
                &utils::format_money(&(buy_price * box_.items_per_box))
            ),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            buy_price /= box_.items_per_box;
            buy_price_changed = true;
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    let mut sell_price = box_.product.sell_price;
    loop {
        utils::printline(terminal_io, "\r\nEnter box sellprice.");
        if buy_price_changed {
            let margin = rv_api::get_margin(&credentials).unwrap();
            let margin_pretty = format!("{}%", (margin * 100.0).ceil());
            sell_price = (buy_price as f32 * (1.0 + margin)).ceil() as i32;
            utils::printline(
                terminal_io,
                &format!(
                    "Suggest {} calculated with the margin of {}",
                    &utils::format_money(&(sell_price * box_.items_per_box)),
                    margin_pretty
                ),
            );
        }
        utils::printline(
            terminal_io,
            &format!(
                "Modify or keep [{}]:",
                &utils::format_money(&(sell_price * box_.items_per_box)),
            ),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            if buy_price_changed {
                printline(terminal_io, "Using the suggested price.");
            } else {
                printline(terminal_io, "Nothing changed.");
            }
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            buy_price /= box_.items_per_box;
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");
    let box_count = loop {
        utils::printline(terminal_io, "Enter how many boxes to add. Format: [0-9]+");
        utils::printline(terminal_io, &format!("Modify or keep [0]:"));
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            break 0;
        } else if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            break input_line.parse().unwrap();
        } else {
            print_error_line(terminal_io, "Invalid stock entered, please retry!\n");
        }
    };
    printline(terminal_io, "");
    if box_count == 0 {
        printline(terminal_io, "Added 0 boxes.");
        return TimeoutResult::RESULT(());
    }

    match rv_api::buy_in_box(barcode, buy_price, sell_price, box_count, credentials).unwrap() {
        ApiResult::Success => utils::printline(
            terminal_io,
            &format!(
                "Added {} boxes. Total of {} items.",
                box_count,
                box_.items_per_box * box_count
            ),
        ),
        ApiResult::Fail(msg) => print_error_line(terminal_io, &msg),
    }

    TimeoutResult::RESULT(())
}

fn buy_in_product(
    barcode: &str,
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    let product;
    match rv_api::get_product_info_admin(&credentials, barcode).unwrap() {
        ApiResultValue::Success(suc) => product = suc,
        ApiResultValue::Fail(msg) => {
            utils::print_error_line(terminal_io, &msg);
            return TimeoutResult::RESULT(());
        }
    };
    utils::printline(terminal_io, &format!("Adding new products to stock."));
    let mut buy_price = product.buy_price;
    let mut buy_price_changed = false;
    loop {
        utils::printline(
            terminal_io,
            "Enter item buyprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
        utils::printline(
            terminal_io,
            &format!("Modify or keep [{}]:", &utils::format_money(&buy_price)),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            buy_price_changed = true;
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    let mut sell_price = product.sell_price;
    loop {
        utils::printline(terminal_io, "\r\nEnter item sellprice.");
        if buy_price_changed {
            let margin = rv_api::get_margin(&credentials).unwrap();
            let margin_pretty = format!("{}%", (margin * 100.0).ceil());
            sell_price = (buy_price as f32 * (1.0 + margin)).ceil() as i32;
            utils::printline(
                terminal_io,
                &format!(
                    "Suggest {} calculated with the margin of {}",
                    &utils::format_money(&sell_price),
                    margin_pretty
                ),
            );
            utils::printline(
                terminal_io,
                &format!("Modify or keep [{}]:", &utils::format_money(&sell_price)),
            );
        } else {
            utils::printline(
                terminal_io,
                &format!("Modify or keep [{}]:", &utils::format_money(&sell_price)),
            );
        }
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            if buy_price_changed {
                printline(terminal_io, "Using the suggested price.");
            } else {
                printline(terminal_io, "Nothing changed.");
            }
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");
    let count = loop {
        utils::printline(terminal_io, "How many products to add? Format: [0-9]+");
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            break input_line.parse().unwrap();
        } else {
            print_error_line(terminal_io, "Invalid count entered, please retry!\n");
        }
    };
    printline(terminal_io, "");
    if count == 0 {
        printline(terminal_io, "Added 0 products to stock.");
        return TimeoutResult::RESULT(());
    }

    rv_api::buy_in_product(barcode, buy_price, sell_price, count, credentials);
    utils::printline(terminal_io, &format!("Added {} products to stock.", count));
    TimeoutResult::RESULT(())
}

fn return_purchase(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    utils::print_title(terminal_io, "Return recent purchase");

    utils::printline(terminal_io, "Enter product barcode:");
    let barcode = match readline(terminal_io, INPUT_TIMEOUT_SHORT) {
        TimeoutResult::RESULT(s) => {
            if Regex::new("^[0-9]+$").unwrap().is_match(&s) {
                s
            } else {
                print_error_line(terminal_io, "Invalid barcode!");

                std::thread::sleep(std::time::Duration::from_millis(2000));
                return TimeoutResult::RESULT(());
            }
        }
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
    };

    match return_product(credentials, &barcode).unwrap() {
        ApiResult::Success => {
            let product = get_product_info(credentials, &barcode).unwrap();
            printline(
                terminal_io,
                &format!("Returned product: {} successfully", product.name),
            );
        }
        ApiResult::Fail(msg) => print_error_line(terminal_io, &format!("Return failed {msg}")),
    }
    TimeoutResult::RESULT(())
}

fn multibuy(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Multibuy");

    utils::printline(terminal_io, "Enter item barcode:");
    let barcode = match readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::RESULT(s) => {
            if Regex::new("^[0-9]+$").unwrap().is_match(&s) {
                s
            } else {
                print_error_line(terminal_io, "Invalid barcode!");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return TimeoutResult::RESULT(());
            }
        }
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
    };

    utils::printline(terminal_io, "Enter item count to buy:");
    let count: i32 = match readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::RESULT(s) => {
            if Regex::new("^[1-9][0-9]*$").unwrap().is_match(&s) {
                s.parse().unwrap()
            } else {
                print_error_line(terminal_io, "Invalid count!");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return TimeoutResult::RESULT(());
            }
        }
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
    };
    purchase_items(terminal_io, &barcode, count, credentials);
    TimeoutResult::RESULT(())
}
fn purchase_items(
    terminal_io: &mut TerminalIO,
    barcode: &str,
    count: i32,
    credentials: &rv_api::AuthenticationResponse,
) {
    match rv_api::purchase_item(&credentials, &barcode, &count).unwrap() {
        ApiResult::Success => {
            let product_info = rv_api::get_product_info(&credentials, &barcode).unwrap();
            utils::printline(
                terminal_io,
                &format!(
                    "Bought {}x {} ({}EUR) Total ({}EUR)",
                    count,
                    product_info.name,
                    utils::format_money(&product_info.price),
                    utils::format_money(&(count * product_info.price))
                ),
            );
        }
        ApiResult::Fail(msg) => {
            print_error_line(terminal_io, &format!("Purchase failed: {msg}"));
        }
    }
}
fn change_user_rfid(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Set login RFID");
    utils::printline(
        terminal_io,
        "Scan RFID to use for logging in. ENTER to cancel.",
    );
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
            Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
            Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                KeyCode::Enter => {
                    utils::printline(terminal_io, "RFID change cancelled");
                    return TimeoutResult::RESULT(());
                }
                _ => (),
            },
            Ok(input::InputEvent::Rfid(rfid)) => {
                rv_api::change_rfid(credentials, &rfid).unwrap();
                utils::printline(terminal_io, "RFID changed successfully");
                return TimeoutResult::RESULT(());
            }
            _ => return TimeoutResult::RESULT(()),
        }
    }
}

fn search_products(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Product search");
    printline(terminal_io, "Enter name or barcode");
    let query = match readline(terminal_io, INPUT_TIMEOUT_SHORT) {
        TimeoutResult::RESULT(s) => s,
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
    };
    let product_results = rv_api::search_products(credentials, &query).unwrap();
    let user_info = get_user_info(credentials).unwrap();
    let box_results = match user_info.is_admin() {
        true => rv_api::search_boxes(credentials, &query).unwrap(),
        false => Vec::new(),
    };
    if product_results.is_empty() && box_results.is_empty() {
        utils::printline(
            terminal_io,
            &format!("No results found with query {}", &query),
        );
        return TimeoutResult::RESULT(());
    }
    printline(terminal_io, "\r\nResult products:");
    let mut dupehack: Vec<String> = Vec::new();
    for product in product_results {
        dupehack.push(format!(
            "{}, {} EUR, ID: {}, {} in stock.",
            product.name,
            utils::format_money(&product.price),
            product.barcode,
            product.stock
        ));
    }
    for product in box_results.iter().map(|f| &f.product) {
        dupehack.push(format!(
            "{}, {} EUR, ID: {}, {} in stock.",
            product.name,
            utils::format_money(&product.sell_price),
            product.barcode,
            product.stock
        ));
    }
    dupehack.sort();
    dupehack.dedup();
    for line in dupehack {
        printline(terminal_io, &line);
    }
    if user_info.is_admin() {
        printline(terminal_io, "\r\nResult boxes:");
        for box_result in box_results {
            utils::printline(
                terminal_io,
                &format!(
                    "{} containing {}x of {} {}",
                    box_result.box_barcode,
                    box_result.items_per_box,
                    box_result.product.barcode,
                    box_result.product.name,
                ),
            );
        }
    }
    TimeoutResult::RESULT(())
}

fn change_user_password_user(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change password");

    execute!(terminal_io.writer, Print("Enter new password: ")).unwrap();
    let password1;
    match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password1 = s,
    }

    utils::printline(terminal_io, "");
    execute!(terminal_io.writer, Print("Enter new password again: ")).unwrap();
    let password2;
    match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password2 = s,
    }
    utils::printline(terminal_io, "");

    if password1.len() == 0 {
        utils::printline(
            terminal_io,
            "Empty password is not allowed! Password not changed.",
        );
    } else if password1 == password2 {
        match rv_api::change_password(credentials, &password1).unwrap() {
            rv_api::ApiResult::Success => {
                utils::printline(terminal_io, "Password successfully changed.");
            }
            rv_api::ApiResult::Fail(msg) => {
                utils::print_error_line(terminal_io, &format!("Password change failed: {msg}"));
            }
        }
    } else {
        utils::printline(terminal_io, "Passwords do not match! Password not changed.");
    }
    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn change_user_password_admin(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change password (admin)");
    execute!(terminal_io.writer, Print("Enter username: ")).unwrap();
    let username;
    match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => username = s,
    }

    let user = match rv_api::get_user_info_by_username(credentials, &username).unwrap() {
        ApiResultValue::Fail(msg) => {
            print_error_line(terminal_io, &msg);
            utils::printline(terminal_io, "");
            return TimeoutResult::RESULT(());
        }
        ApiResultValue::Success(user) => user,
    };

    execute!(terminal_io.writer, Print("Enter new password: ")).unwrap();
    let password1;
    match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password1 = s,
    }

    utils::printline(terminal_io, "");
    execute!(terminal_io.writer, Print("Enter new password again: ")).unwrap();
    let password2;
    match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password2 = s,
    }
    utils::printline(terminal_io, "");

    if password1.len() == 0 {
        utils::printline(
            terminal_io,
            "Empty password is not allowed! Password not changed.",
        );
    } else if password1 == password2 {
        match rv_api::change_password_admin(credentials, user.user_id, &password1).unwrap() {
            rv_api::ApiResult::Success => {
                utils::printline(terminal_io, "Password successfully changed.");
            }
            rv_api::ApiResult::Fail(msg) => {
                utils::print_error_line(terminal_io, &format!("Password change failed: {msg}"));
            }
        }
    } else {
        utils::printline(terminal_io, "Passwords do not match! Password not changed.");
    }
    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn deposit(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Deposit money");
    utils::printline(
        terminal_io,
        "How much to deposit? Format: [0-9]+\\.[0-9][0-9]",
    );
    utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
    let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };
    if !Regex::new("^[0-9]+\\.[0-9][0-9]$")
        .unwrap()
        .is_match(&input_line)
    {
        printline(terminal_io, "");
        utils::print_error_line(terminal_io, "Invalid input. Deposit aborted!");
        return TimeoutResult::RESULT(());
    }
    let amount: u32 = input_line.replace(".", "").parse().unwrap();

    if amount > 25000 {
        printline(terminal_io, "");
        utils::print_error_line(
            terminal_io,
            "You can deposit at most 250 EUR at once. Deposit aborted!",
        );
        return TimeoutResult::RESULT(());
    }

    let amount_formatted = utils::format_money(&(amount as i32));
    execute!(
        terminal_io.writer,
        Print("\r\n"),
        PrintStyledContent(
            "PLEASE NOTE: WITHDRAWING MONEY IS NOT POSSIBLE."
                .with(Color::Black)
                .on(Color::White)
        ),
        Print("\r\n"),
        Print(&format!(
            "\
            You can't transfer money to somebody else's account.\r\n\
            Please confirm your deposit of {} euros.\r\n\
            ",
            amount_formatted
        )),
        Print("PLEASE TYPE '"),
        PrintStyledContent(
            amount_formatted
                .to_string()
                .with(Color::Black)
                .on(Color::White)
        ),
        Print("' FOLLOWED BY <ENTER>: ")
    )
    .unwrap();

    match utils::readline(terminal_io, INPUT_TIMEOUT_SHORT) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "\r\nTimed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => {
            if s.len() == 0 {
                utils::printline(terminal_io, "\r\nDeposit aborted! Cancelled by user.");
                return TimeoutResult::RESULT(());
            } else if s != amount_formatted {
                utils::print_error_line(
                    terminal_io,
                    "\r\nDeposit aborted! Given amounts do not match.",
                );
                return TimeoutResult::RESULT(());
            }
        }
    };

    loop {
        execute!(
            terminal_io.writer,
            Print("\r\n"),
            Print("Did you deposit money as cash or via banktransfer?\r\n"),
            Print("PLEASE TYPE EITHER '"),
            PrintStyledContent("cash".with(Color::Black).on(Color::White)),
            Print("' OR '"),
            PrintStyledContent("banktransfer".with(Color::Black).on(Color::White)),
            Print("' FOLLOWED BY <ENTER>:\r\n"),
        )
        .unwrap();
        match utils::readline(terminal_io, INPUT_TIMEOUT_SHORT) {
            TimeoutResult::TIMEOUT => {
                utils::printline(terminal_io, "\r\nTimed out!");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return TimeoutResult::TIMEOUT;
            }
            TimeoutResult::RESULT(s) => {
                if s.len() == 0 {
                    utils::printline(terminal_io, "\r\nDeposit aborted! Cancelled by user.");
                    return TimeoutResult::RESULT(());
                } else if s == "cash" {
                    rv_api::deposit(&credentials, &amount, "cash").unwrap();
                    break;
                } else if s == "banktransfer" {
                    rv_api::deposit(&credentials, &amount, "banktransfer").unwrap();
                    break;
                } else {
                    print_error_line(terminal_io, "Invalid deposit type entered!");
                }
            }
        };
    }

    utils::printline(
        terminal_io,
        &format!(
            "\r\nDeposited {} EUR.",
            utils::format_money(&(amount as i32))
        ),
    );
    TimeoutResult::RESULT(())
}

fn change_item_properties(
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change item properties");
    utils::printline(terminal_io, "Enter barcode:");
    let barcode = match readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::RESULT(s) => {
            if Regex::new("^[0-9]+$").unwrap().is_match(&s) {
                s
            } else {
                print_error_line(terminal_io, "invalid barcode!");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return TimeoutResult::RESULT(());
            }
        }
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
    };

    let product = match rv_api::get_product_info_admin(&credentials, &barcode).unwrap() {
        ApiResultValue::Success(product) => Some(product),
        ApiResultValue::Fail(msg) => {
            utils::print_error_line(terminal_io, &msg);
            None
        }
    };
    if product.is_some() {
        return change_product_properties(credentials, terminal_io, &barcode);
    }
    if let Some(b) = rv_api::get_box_info_admin(&barcode, credentials).unwrap() {
        return change_box_properties(credentials, terminal_io, b.product.barcode);
    }
    utils::print_error_line(terminal_io, "No matching box or product found!");
    TimeoutResult::RESULT(())
}

fn change_box_properties(
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
    barcode: String,
) -> TimeoutResult<()> {
    let box_result = match rv_api::get_box_info_admin(&barcode, &credentials).unwrap() {
        Some(b) => b,
        None => {
            print_error_line(terminal_io, &format!("No box found with {barcode}"));
            return TimeoutResult::RESULT(());
        }
    };
    let mut product_barcode = box_result.product.barcode;
    printline(
        terminal_io,
        &format!("Current itembarcode: '{product_barcode}'"),
    );
    printline(terminal_io, &format!("Modify or keep [{product_barcode}]:"));
    let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };
    if input_line.len() > 0 {
        match get_product_info(credentials, &input_line) {
            Some(_) => product_barcode = input_line,
            None => match get_box_info_admin(&input_line, credentials).unwrap() {
                Some(_) => {
                    print_error_line(terminal_io, "Box with the given barcode already exists!");
                    return TimeoutResult::RESULT(());
                }
                None => {
                    if let TimeoutResult::TIMEOUT =
                        new_product(&input_line, credentials, terminal_io)
                    {
                        return TimeoutResult::TIMEOUT;
                    }
                    if get_product_info(credentials, &input_line).is_none() {
                        print_error_line(terminal_io, "Adding new item failed!");
                        return TimeoutResult::RESULT(());
                    }
                    product_barcode = input_line;
                }
            },
        }
    } else {
        printline(terminal_io, "Nothing changed.");
    }

    let mut items_per_box = box_result.items_per_box;
    loop {
        printline(
            terminal_io,
            &format!("Current items per box: '{items_per_box}'"),
        );
        printline(
            terminal_io,
            &format!("Modify or keep [{items_per_box}] Format: [0-9]+:"),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else {
            if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
                items_per_box = input_line.parse().unwrap();
                break;
            } else {
                print_error_line(terminal_io, "Invalid number entered, please retry!\n");
            }
        }
    }
    match update_box(&barcode, items_per_box, &product_barcode, credentials) {
        ApiResult::Success => printline(terminal_io, "Box modified successfully."),
        ApiResult::Fail(msg) => {
            print_error_line(terminal_io, &format!("Modifying box failed: {msg}"))
        }
    }
    printline(terminal_io, "");

    TimeoutResult::RESULT(())
}
fn change_product_properties(
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
    barcode: &str,
) -> TimeoutResult<()> {
    let product = match rv_api::get_product_info_admin(&credentials, &barcode).unwrap() {
        ApiResultValue::Success(product) => product,
        ApiResultValue::Fail(msg) => {
            utils::print_error_line(terminal_io, &msg);
            return TimeoutResult::RESULT(());
        }
    };
    let mut name = product.name;
    utils::printline(terminal_io, &format!("Current description: '{name}'"));
    utils::printline(terminal_io, &format!("Modify or keep [{name}]:"));

    let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };
    if input_line.len() > 0 {
        name = input_line.to_string();
    } else {
        printline(terminal_io, "Nothing changed.");
    }

    printline(terminal_io, "");

    let mut buy_price = product.buy_price;
    let mut buy_price_changed = false;
    loop {
        utils::printline(
            terminal_io,
            "Please enter item buyprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");
        utils::printline(
            terminal_io,
            &format!("Modify or keep [{}]:", &utils::format_money(&buy_price)),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            buy_price_changed = true;
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    let mut sell_price = product.sell_price;
    loop {
        utils::printline(terminal_io, "\r\nPlease enter item sellprice.");
        if buy_price_changed {
            let margin = rv_api::get_margin(&credentials).unwrap();
            let margin_pretty = format!("{}%", (margin * 100.0).ceil());
            sell_price = (buy_price as f32 * (1.0 + margin)).ceil() as i32;
            utils::printline(
                terminal_io,
                &format!(
                    "Suggest {} calculated with the margin of {}",
                    &utils::format_money(&sell_price),
                    margin_pretty
                ),
            );
        }
        utils::printline(
            terminal_io,
            &format!("Modify or keep [{}]:", &utils::format_money(&sell_price)),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            if buy_price_changed {
                printline(terminal_io, "Using the suggested price.");
            } else {
                printline(terminal_io, "Nothing changed.");
            }
            break;
        } else if Regex::new("^[0-9]+\\.[0-9][0-9]$")
            .expect("")
            .is_match(&input_line)
        {
            buy_price = input_line.replace(".", "").parse().unwrap();
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");
    let mut stock = product.stock;
    loop {
        utils::printline(terminal_io, "Please enter item stock. Format: (+|-)?[0-9]+");
        utils::printline(terminal_io, "No prefix or - means to set stock to negative or positive number given. + prefix means to increment stock e.g. +5 when stock is 2 results in stock of 7.");
        utils::printline(terminal_io, &format!("Modify or keep [{stock}]:"));
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else if Regex::new("^(\\+|-)?[0-9]+")
            .expect("")
            .is_match(&input_line)
        {
            if input_line.chars().nth(0).unwrap() == '+' {
                stock = stock + input_line.parse::<i32>().unwrap();
            } else {
                stock = input_line.parse().unwrap();
            }
            break;
        } else {
            print_error_line(terminal_io, "Invalid stock entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    let mut category = product.category;
    loop {
        utils::printline(terminal_io, "Please enter product category id.");
        utils::printline(terminal_io, "Categories available:");
        let categories = rv_api::get_categories(&credentials).unwrap();
        for category in categories.iter() {
            utils::printline(
                terminal_io,
                &format!("{}, id: {}", category.description, category.category_id),
            );
        }
        utils::printline(
            terminal_io,
            &format!("Modify or keep [{}]:", category.description),
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break;
        } else if Regex::new("^[0-9]+").expect("").is_match(&input_line) {
            let chosen: i32 = input_line.parse().unwrap();
            match categories.iter().find(|c| c.category_id == chosen) {
                Some(c) => {
                    category = ProductCategory {
                        description: c.description.clone(),
                        category_id: c.category_id,
                    };
                }
                None => {
                    print_error_line(terminal_io, "Invalid category id entered, please retry!\n");
                    continue;
                }
            }
            break;
        } else {
            print_error_line(terminal_io, "Invalid category entered, please retry!\n");
        }
    }
    rv_api::update_product(
        &barcode,
        &name,
        category.category_id,
        buy_price,
        sell_price,
        stock,
        credentials,
    )
    .unwrap();
    utils::printline(terminal_io, "Product updated.");
    TimeoutResult::RESULT(())
}

fn management_mode_loop(
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) -> TimeoutResult<()> {
    clear_terminal(terminal_io);
    'main: loop {
        let user_info = rv_api::get_user_info(&credentials).unwrap();

        queue!(
            terminal_io.writer,
            cursor::MoveTo(0, terminal::size()?.1),
            PrintStyledContent(RV_LOGO.yellow()),
            Print("=== management mode ===\r\n"),
            PrintStyledContent("<barcode>".dark_green().bold()),
            Print(" - IF FOUND update price and count ELSE add as a new item/box\r\n"),
            PrintStyledContent("F".dark_green().bold()),
            Print(" - list matching products\r\n"),
            PrintStyledContent("I".dark_green().bold()),
            Print(" - update all item/box properties\r\n"),
            PrintStyledContent("P".dark_green().bold()),
            Print(" - change password of an user\r\n"),
            PrintStyledContent("<enter>".dark_green().bold()),
            Print(" - exit management mode\r\n"),
        )
        .unwrap();
        execute!(
            terminal_io.writer,
            Print(&format!(
                "\nDear {}, your saldo is {} > ",
                user_info.username,
                utils::format_money(&user_info.money_balance)
            ))
        )
        .unwrap();

        let mut command = String::new();
        loop {
            match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_LONG) {
                Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
                Ok(InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                    KeyCode::Char(c) => match c {
                        'f' => {
                            printline(terminal_io, "\n");
                            match search_products(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "\n");
                            break;
                        }
                        'i' => {
                            printline(terminal_io, "");
                            match change_item_properties(&credentials, terminal_io) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            continue 'main;
                        }
                        'p' => {
                            printline(terminal_io, "\n");

                            match change_user_password_admin(terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        '0'..='9' => {
                            terminal_io.writer.execute(Print(c)).unwrap();
                            command.push(c);
                        }
                        _ => {}
                    },
                    KeyCode::Backspace => {
                        if !command.is_empty() {
                            execute!(
                                terminal_io.writer,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )
                            .unwrap();
                            command.pop();
                        }
                    }
                    KeyCode::Enter => {
                        command = command.trim().to_string();
                        utils::printline(terminal_io, "\r\n");
                        if command.is_empty() {
                            clear_terminal(terminal_io);
                            break 'main;
                        } else if Regex::new("^[0-9]+$").expect("").is_match(&command) {
                            let barcode = &command;
                            if let Some(_) = rv_api::get_product_info(credentials, barcode) {
                                match buy_in_product(barcode, credentials, terminal_io) {
                                    TimeoutResult::RESULT(_) => (),
                                    TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                }
                                break;
                            }
                            if let Some(_) =
                                rv_api::get_box_info_admin(barcode, credentials).unwrap()
                            {
                                match buy_in_box(barcode, credentials, terminal_io) {
                                    TimeoutResult::RESULT(_) => (),
                                    TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                }
                                break;
                            }
                            print_error_line(
                                terminal_io,
                                &format!("No box or product found with barcode {barcode}"),
                            );
                            new_item(&command, credentials, terminal_io);
                            break;
                        } else {
                            utils::print_error_line(
                                terminal_io,
                                &format!("unknown command: {}\r\n", &command),
                            );
                            break;
                        }
                    }
                    _ => (),
                },
                Ok(InputEvent::Rfid(_)) => {
                    // Logout
                    return TimeoutResult::RESULT(());
                }
                _ => (),
            }
        }
    }
    TimeoutResult::RESULT(())
}

fn print_user_loop_instructions(
    credentials: &rv_api::AuthenticationResponse,
    terminal_io: &mut TerminalIO,
) {
    let user_info = rv_api::get_user_info(&credentials).unwrap();
    queue!(
        terminal_io.writer,
        cursor::MoveTo(0, terminal::size()?.1),
        Print(RV_LOGO.yellow()),
        Print("Available commands (press key to select):\r\n"),
        PrintStyledContent("<barcode>".dark_green()),
        Print(" - buy this item\r\n"),
        PrintStyledContent("B".dark_green().bold()),
        Print(" - buy item multiple times\r\n"),
        PrintStyledContent("D".dark_green().bold()),
        Print(" - deposit to your account\r\n"),
        PrintStyledContent("F".dark_green().bold()),
        Print(" - list matching products\r\n"),
        PrintStyledContent("P".dark_green().bold()),
        Print(" - change password\r\n"),
        PrintStyledContent("R".dark_green().bold()),
        Print(" - manage your rfid\r\n"),
        PrintStyledContent("U".dark_green().bold()),
        Print(" - undo a recent purchase\r\n"),
        PrintStyledContent("<enter>".dark_green().bold()),
        Print(" - log out\r\n"),
    )
    .unwrap();
    if user_info.is_admin() {
        queue!(
            terminal_io.writer,
            PrintStyledContent("M".dark_green().bold()),
            Print(" - enter management mode\r\n"),
        )
        .unwrap();
    }
}

pub fn user_loop(credentials: &rv_api::AuthenticationResponse, terminal_io: &mut TerminalIO) {
    execute!(
        terminal_io.writer,
        terminal::Clear(terminal::ClearType::All)
    )
    .unwrap();
    print_user_loop_instructions(credentials, terminal_io);
    'main: loop {
        let user_info = rv_api::get_user_info(&credentials).unwrap();
        execute!(
            terminal_io.writer,
            Print(&format!(
                "\nDear {}, your saldo is {} > ",
                user_info.username,
                utils::format_money(&user_info.money_balance)
            ))
        )
        .unwrap();

        let mut command = String::new();
        loop {
            match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
                Err(RecvTimeoutError::Timeout) => {
                    utils::printline(terminal_io, "Timed out!");
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    break 'main;
                }
                Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                    KeyCode::Char(c) => match c {
                        'b' => {
                            printline(terminal_io, "\n");
                            match multibuy(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'd' => {
                            printline(terminal_io, "\n");
                            match deposit(terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "\n");
                            break;
                        }
                        'f' => {
                            printline(terminal_io, "\n");
                            match search_products(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "\n");
                            break;
                        }
                        'p' => {
                            printline(terminal_io, "\n");
                            match change_user_password_user(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "\n");
                            break;
                        }
                        'r' => {
                            printline(terminal_io, "\n");
                            match change_user_rfid(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "\n");
                            break;
                        }
                        'm' => {
                            if user_info.is_admin() {
                                printline(terminal_io, "\n");
                                match management_mode_loop(credentials, terminal_io) {
                                    TimeoutResult::TIMEOUT => break 'main,
                                    _ => (),
                                }
                                print_user_loop_instructions(credentials, terminal_io);
                                break;
                            }
                        }
                        'u' => {
                            printline(terminal_io, "\n");
                            match return_purchase(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            break;
                        }
                        'q' => {
                            // Legacy behavior wanted by some old users, need not to show in the list of commands
                            break 'main; // Logout
                        }
                        '0'..='9' => {
                            terminal_io.writer.execute(Print(c)).unwrap();
                            command.push(c);
                        }
                        _ => (),
                    },
                    KeyCode::Backspace => {
                        if !command.is_empty() {
                            execute!(
                                terminal_io.writer,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )
                            .unwrap();
                            command.pop();
                        }
                    }
                    KeyCode::Enter => {
                        command = command.trim().to_string();
                        utils::printline(terminal_io, "\r\n");
                        if command.is_empty() {
                            break 'main; // Logout
                        } else if command == "exit" {
                            disable_raw_mode().unwrap();
                            exit(0);
                        } else if Regex::new("^[0-9]+$").expect("").is_match(&command) {
                            purchase_items(terminal_io, &command, 1, credentials);
                            printline(terminal_io, "\n");
                            break;
                        } else {
                            utils::print_error_line(
                                terminal_io,
                                &format!("unknown command: {}\r\n", &command),
                            );
                            break;
                        }
                    }
                    _ => (),
                },
                Ok(InputEvent::Rfid(_)) => {
                    // Logout
                    return;
                }
                _ => (),
            };
        }
    }
}
