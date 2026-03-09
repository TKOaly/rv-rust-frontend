use super::user;

use crate::input;
use crate::rv_api;
use crate::rv_api::get_box_info_admin;
use crate::rv_api::get_product_info;
use crate::rv_api::update_box;
use crate::rv_api::ApiResultValue;
use crate::rv_api::ProductCategory;
use crate::rv_api::UserInfo;
use crate::utils;
use crate::utils::clear_terminal;
use crate::utils::print_error_line;
use crate::utils::print_title;
use crate::utils::printline;
use crate::utils::readline;
use crate::utils::TimeoutResult;
use crate::TerminalIO;
use crate::INPUT_TIMEOUT_LONG;

use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute, queue,
    style::{Print, PrintStyledContent, Stylize},
    terminal, ExecutableCommand,
};
use input::InputEvent;
use regex::Regex;
use rv_api::ApiResult;
use std::{sync::mpsc::RecvTimeoutError, time::Duration};
use user::{buy_in_box, buy_in_product, search_products};

fn input_calculator(input: &str) -> Option<i32> {
    if input.is_empty() {
        return None;
    }

    let numbers = input.split("*");
    let mut product: Option<i32> = None;

    for number in numbers {
        let num1 = number.parse::<i32>().ok()?;
        product = Some(match product {
            Some(num2) => num2 * num1,
            None => num1,
        });
    }

    product
}

fn new_product(
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
        let margin = rv_api::get_margin(&credentials).unwrap() as f64;
        let margin_pretty = format!("{}%", (margin * 100.0).ceil());
        let suggested_price = (buy_price as f64 * (1.0 + margin)).ceil() as i32;
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
        utils::printline(
            terminal_io,
            "Enter item stock. Format: [0-9]+ or [0.9]+\\*[0.9]+",
        );
        utils::printline(terminal_io, &format!("Modify or keep [{suggested_stock}]"));
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };
        if input_line.len() == 0 {
            printline(terminal_io, "Nothing changed.");
            break suggested_stock;
        }
        match input_calculator(&input_line) {
            Some(stock) => {
                break stock;
            }
            None => {
                print_error_line(terminal_io, "Invalid stock entered, please retry!\n");
            }
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
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
                        new_product(&product_barcode, terminal_io, credentials)
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
            return buy_in_box(barcode, terminal_io, credentials);
        }
        ApiResult::Fail(msg) => print_error_line(terminal_io, &msg),
    }
    TimeoutResult::RESULT(())
}

fn new_item(
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
                KeyCode::Char(c) => match c.to_ascii_lowercase() {
                    'b' => {
                        utils::printline(terminal_io, "");
                        return new_box(barcode, terminal_io, credentials);
                    }
                    'p' => {
                        utils::printline(terminal_io, "");
                        return new_product(barcode, terminal_io, credentials);
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
    }
}

fn change_item_properties(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
        return change_product_properties(&barcode, terminal_io, credentials);
    }
    if let Some(b) = rv_api::get_box_info_admin(&barcode, credentials).unwrap() {
        return change_box_properties(b.product.barcode, terminal_io, credentials);
    }
    utils::print_error_line(terminal_io, "No matching box or product found!");
    TimeoutResult::RESULT(())
}

fn change_box_properties(
    barcode: String,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
                        new_product(&input_line, terminal_io, credentials)
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
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
            let margin = rv_api::get_margin(&credentials).unwrap() as f64;
            let margin_pretty = format!("{}%", (margin * 100.0).ceil());
            sell_price = (buy_price as f64 * (1.0 + margin)).ceil() as i32;
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
            sell_price = input_line.replace(".", "").parse().unwrap();
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

fn change_user_password_admin(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change password (admin)");
    execute!(terminal_io.writer, Print("Enter username: ")).unwrap();
    let username;
    match utils::readline(terminal_io, timeout) {
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
    match utils::readpasswd(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password1 = s,
    }

    utils::printline(terminal_io, "");
    execute!(terminal_io.writer, Print("Enter new password again: ")).unwrap();
    let password2;
    match utils::readpasswd(terminal_io, timeout) {
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

fn generate_temp_password_admin(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Generate temporary password for user");
    execute!(terminal_io.writer, Print("Enter username: ")).unwrap();
    let username;
    match utils::readline(terminal_io, timeout) {
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

    match rv_api::generate_temp_password(credentials, user.user_id).unwrap() {
        rv_api::ApiResult::Success => {
            utils::printline(
                terminal_io,
                &format!("Temperary password successfully for {}.", user.username),
            );
        }
        rv_api::ApiResult::Fail(msg) => {
            utils::print_error_line(terminal_io, &format!("Password change failed: {msg}"));
        }
    }

    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn search_for_user(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(
        terminal_io,
        "Search for user whit email or user's real name",
    );
    execute!(
        terminal_io.writer,
        Print("Enter email or  user's real name: ")
    )
    .unwrap();

    let input = match utils::readline(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };

    let mut users: Vec<UserInfo> = Vec::new();

    if input.split("@").count() == 2 {
        let user = match rv_api::get_user_info_by_email(credentials, &input).unwrap() {
            ApiResultValue::Fail(msg) => {
                print_error_line(terminal_io, &msg);
                utils::printline(terminal_io, "");
                return TimeoutResult::RESULT(());
            }
            ApiResultValue::Success(user) => user,
        };
        users.push(user);
    } else {
        let user = match rv_api::get_user_info_by_full_name(credentials, &input).unwrap() {
            ApiResultValue::Fail(msg) => {
                print_error_line(terminal_io, &msg);
                utils::printline(terminal_io, "");
                return TimeoutResult::RESULT(());
            }
            ApiResultValue::Success(user) => user,
        };
        users.push(user);
    }
    utils::printline(terminal_io, "");
    utils::print_title(terminal_io, "Found users:");
    for user in users {
        utils::printline(
            terminal_io,
            &format!(
                "username: {} email: {} full name: {}",
                user.username, user.email, user.full_name
            ),
        );
    }
    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn process_barcode_admin(
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    if let Some(_) = rv_api::get_product_info(credentials, barcode) {
        match buy_in_product(barcode, terminal_io, credentials) {
            TimeoutResult::RESULT(_) => return TimeoutResult::RESULT(()),
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        }
    }
    if let Some(_) = rv_api::get_box_info_admin(barcode, credentials).unwrap() {
        match buy_in_box(barcode, terminal_io, credentials) {
            TimeoutResult::RESULT(_) => return TimeoutResult::RESULT(()),
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        }
    }
    print_error_line(
        terminal_io,
        &format!("No box or product found with barcode {barcode}"),
    );
    new_item(barcode, terminal_io, credentials);
    return TimeoutResult::RESULT(());
}

pub fn management_mode_loop(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    clear_terminal(terminal_io);
    'main: loop {
        let user_info = rv_api::get_user_info(&credentials).unwrap();

        queue!(
            terminal_io.writer,
            cursor::MoveTo(0, terminal::size()?.1),
            Print("=== management mode ===\r\n"),
            PrintStyledContent("<barcode>".dark_green().bold()),
            Print(" - IF FOUND update price and count ELSE add as a new item/box\r\n"),
            PrintStyledContent("F".dark_green().bold()),
            Print(" - list matching products\r\n"),
            PrintStyledContent("I".dark_green().bold()),
            Print(" - update all item/box properties\r\n"),
            PrintStyledContent("S".dark_green().bold()),
            Print(" - Search for an username\r\n"),
            PrintStyledContent("P".dark_green().bold()),
            Print(" - change password of an user\r\n"),
            PrintStyledContent("E".dark_green().bold()),
            Print(" - generate temppasword and send it to user\r\n"),
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
                    KeyCode::Char(c) => match c.to_ascii_lowercase() {
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
                            match change_item_properties(terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        's' => {
                            printline(terminal_io, "");
                            match search_for_user(terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'p' => {
                            printline(terminal_io, "\n");
                            match change_user_password_admin(
                                INPUT_TIMEOUT_LONG,
                                terminal_io,
                                &credentials,
                            ) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'e' => {
                            printline(terminal_io, "");
                            match generate_temp_password_admin(
                                INPUT_TIMEOUT_LONG,
                                terminal_io,
                                &credentials,
                            ) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'c' => {
                            clear_terminal(terminal_io);
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
                            match process_barcode_admin(&command, terminal_io, credentials) {
                                TimeoutResult::RESULT(_) => (),
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                            }
                            break;
                        } else {
                            utils::print_error_line(
                                terminal_io,
                                &format!("unknown command: {}\r\n", &command),
                            );
                            break;
                        }
                    }
                    KeyCode::F(5) => {
                        break;
                    }
                    _ => (),
                },
                Ok(InputEvent::Barcode(barcode)) => {
                    let trimend_barcode = barcode.trim();
                    if Regex::new("^[0-9]+$").expect("").is_match(trimend_barcode) {
                        match process_barcode_admin(trimend_barcode, terminal_io, credentials) {
                            TimeoutResult::RESULT(_) => (),
                            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                        }
                    }
                    break;
                }
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
