use super::management;
use super::setting;

use crate::input;
use crate::rv_api;
use crate::rv_api::get_product_info;
use crate::rv_api::get_user_info;
use crate::rv_api::return_product;
use crate::rv_api::ApiResultPurchaseItem;
use crate::rv_api::ApiResultPurchaseItemFailType;
use crate::rv_api::ApiResultValue;
use crate::rv_api::UserInfoTrait;
use crate::utils;
use crate::utils::load_ascii;
use crate::utils::print_error_line;
use crate::utils::print_title;
use crate::utils::printline;
use crate::utils::purchase_fail_bell;
use crate::utils::readline;
use crate::utils::TimeoutResult;
use crate::TerminalIO;
use crate::INPUT_TIMEOUT_LONG;
use crate::INPUT_TIMEOUT_SHORT;

use chrono::{DateTime, Local};
use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute, queue,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{self, disable_raw_mode},
    ExecutableCommand,
};
use input::InputEvent;
use regex::Regex;
use rv_api::ApiResult;
use std::process::exit;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;
static PURCHASE_FAILED_MSG1: LazyLock<String> = load_ascii!("../../ascii/purchase_failed.txt");
static PURCHASE_FAILED_MSG2: LazyLock<String> = load_ascii!("../../ascii/purchase_failed2.txt");
static COFFEE_MSG: LazyLock<String> = load_ascii!("../../ascii/netlight.txt");

pub fn buy_in_box(
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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

    utils::printline(
        terminal_io,
        &format!(
            "Current item buyprice: {} x {} = {}",
            &utils::format_money(&buy_price),
            box_.items_per_box,
            &utils::format_money(&(box_.items_per_box * buy_price))
        ),
    );
    loop {
        utils::printline(
            terminal_io,
            "Enter box buyprice. Format: [0-9]+\\.[0-9][0-9]",
        );
        utils::printline(terminal_io, "At least one number, followed by period, followed by two numbers. For example: '1.00', '0.01', '14.42'");

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
            let box_buy_price = input_line.replace(".", "").parse::<i32>().unwrap();
            buy_price = (box_buy_price as f64 / box_.items_per_box as f64).ceil() as i32;
            buy_price_changed = true;

            utils::printline(
                terminal_io,
                &format!(
                    "Calculated new item buyprice: {}",
                    &utils::format_money(&buy_price)
                ),
            );
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");

    let mut sell_price = box_.product.sell_price;
    loop {
        utils::printline(terminal_io, "\r\nEnter item sellprice.");
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

pub fn buy_in_product(
    barcode: &str,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
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
            sell_price = input_line.replace(".", "").parse().unwrap();
            break;
        } else {
            print_error_line(terminal_io, "Invalid price entered, please retry!\n");
        }
    }
    printline(terminal_io, "");
    let count = loop {
        utils::printline(
            terminal_io,
            "How many products to add? Format: [0-9]+ or [0-9]+\\*[0-9]+",
        );
        let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
            TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
            TimeoutResult::RESULT(s) => s,
        };

        if input_line.len() == 0 {
            break 0;
        }

        match utils::calculator_input(&input_line) {
            Some(count) => {
                break count;
            }
            None => {
                print_error_line(terminal_io, "Invalid count entered, please retry!\n");
            }
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
                &format!("\nReturned product: {} successfully", product.name),
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
        ApiResultPurchaseItem::Success => {
            let product_info = rv_api::get_product_info(&credentials, &barcode).unwrap();
            if product_info.barcode == "42615374" {
                // Coffee purchase shill
                utils::printline(terminal_io, &COFFEE_MSG);
            }
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
        ApiResultPurchaseItem::Fail(x) => {
            purchase_fail_bell();
            let user_info = get_user_info(credentials).unwrap();
            utils::set_small_font();
            execute!(
                terminal_io.writer,
                PrintStyledContent(PURCHASE_FAILED_MSG1.to_string().green()),
                PrintStyledContent(PURCHASE_FAILED_MSG2.to_string().red()),
                Print("\r\n"),
                Print(&format!("Dear {}, your purchase has", user_info.username)),
                PrintStyledContent(" FAILED ".red()),
                Print(&format!("with an error: {}\r\n", x.message))
            )
            .unwrap();
            let wait_seconds = match x.error_type {
                ApiResultPurchaseItemFailType::InsufficientFunds => 15,
                _ => 5,
            };
            sleep(Duration::from_secs(wait_seconds));
            Print(format!(
                "You must wait {wait_seconds} seconds before you can proceed!\r\n"
            ));
            while terminal_io.recv.try_recv().is_ok() {
                // Discard all input until channel is empty
            }
            utils::confirm_enter_to_continue(terminal_io);
            utils::set_big_font();
        }
    }
}

pub fn search_products(
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

fn deposit(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Deposit money");
    utils::printline(
        terminal_io,
        "How much to deposit? Format: [0-9]+((\\.|,)[0-9][0-9])?",
    );
    utils::printline(terminal_io, "At least one number, optionally followed by a period or comma followed by two numbers. For example: '1', '0.10', '14,42'");
    let input_line = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => s,
    };
    if !Regex::new("^[0-9]+((\\.|,)[0-9][0-9])?$")
        .unwrap()
        .is_match(&input_line)
    {
        printline(terminal_io, "");
        utils::print_error_line(terminal_io, "Invalid input. Deposit aborted!");
        return TimeoutResult::RESULT(());
    }
    let amount: u32 = if input_line.contains(".") {
        input_line.replace(".", "").parse().unwrap()
    } else if input_line.contains(",") {
        input_line.replace(",", "").parse().unwrap()
    } else {
        // No decimals specified, multiply by 100 to get cents
        input_line.parse::<u32>().unwrap() * 100
    };

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
            } else if s.replace(",", ".") != amount_formatted {
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
            PrintStyledContent("bank".with(Color::Black).on(Color::White)),
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
                    utils::printline(terminal_io, "Remember to put cash in an envelope or send an email immediately to rahastonhoitaja@tko-aly.fi to explain a non-envelope deposit.");
                    utils::printline(
                        terminal_io,
                        &format!("Current date: {}", Local::now().format("%d/%m/%Y")).to_string(),
                    );

                    utils::confirm_enter_to_continue(terminal_io);
                    break;
                } else if s == "bank" {
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

fn print_user_loop_instructions(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) {
    let user_info = rv_api::get_user_info(&credentials).unwrap();
    queue!(
        terminal_io.writer,
        cursor::MoveTo(0, terminal::size()?.1),
        Print("Available commands (press key to select):\r\n"),
        PrintStyledContent("<barcode>".dark_green()),
        Print(" - buy this item\r\n"),
        PrintStyledContent("B".dark_green().bold()),
        Print(" - buy item multiple times\r\n"),
        PrintStyledContent("D".dark_green().bold()),
        Print(" - deposit to your account\r\n"),
        PrintStyledContent("F".dark_green().bold()),
        Print(" - list matching products\r\n"),
        PrintStyledContent("H".dark_green().bold()),
        Print(" - show purchase history\r\n"),
        PrintStyledContent("U".dark_green().bold()),
        Print(" - undo a recent purchase\r\n"),
        PrintStyledContent("S".dark_green().bold()),
        Print(" - change settings\r\n"),
        PrintStyledContent("C".dark_green().bold()),
        Print(" - clear terminal\r\n"),
        PrintStyledContent("<enter>".dark_green().bold()),
        Print(" - log out\r\n"),
    )
    .unwrap();
    utils::print_rv_logo(terminal_io);
    if user_info.is_admin() {
        queue!(
            terminal_io.writer,
            PrintStyledContent("M".dark_green().bold()),
            Print(" - enter management mode\r\n"),
        )
        .unwrap();
    }
}

fn print_user_loop_banner(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) {
    execute!(
        terminal_io.writer,
        terminal::Clear(terminal::ClearType::All)
    )
    .unwrap();
    print_user_loop_instructions(terminal_io, credentials);
    printline(terminal_io, "");
}

pub fn user_loop(terminal_io: &mut TerminalIO, credentials: &rv_api::AuthenticationResponse) {
    print_user_loop_banner(terminal_io, credentials);

    'main: loop {
        let user_info = rv_api::get_user_info(&credentials).unwrap();
        execute!(
            terminal_io.writer,
            Print(&format!(
                "Dear {}, your saldo is {} > ",
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
                    KeyCode::Char(c) => match c.to_ascii_lowercase() {
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
                            printline(terminal_io, "");
                            break;
                        }
                        'f' => {
                            printline(terminal_io, "\n");
                            match search_products(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'h' => {
                            printline(terminal_io, "\n");
                            print_title(terminal_io, "Recent purchases");
                            let mut events = rv_api::purchase_history(credentials);
                            events.sort_by(|a, b| b.time.cmp(&a.time));
                            events.iter().take(10).rev().for_each(|event| {
                                printline(
                                    terminal_io,
                                    &format!(
                                        "{}{}{} {}€",
                                        DateTime::parse_from_rfc3339(&event.time)
                                            .unwrap()
                                            .with_timezone(&chrono_tz::Europe::Helsinki)
                                            .format("%d/%m/%Y %H:%M"),
                                        if event.returned {
                                            " bought [returned] "
                                        } else {
                                            " bought "
                                        },
                                        event.product.name,
                                        utils::format_money(&event.price)
                                    ),
                                )
                            });
                            printline(terminal_io, "");
                            break;
                        }
                        'm' => {
                            if user_info.is_admin() {
                                printline(terminal_io, "\n");
                                match management::management_mode_loop(terminal_io, credentials) {
                                    TimeoutResult::TIMEOUT => break 'main,
                                    _ => (),
                                }
                                print_user_loop_instructions(terminal_io, credentials);
                                break;
                            }
                        }
                        's' => {
                            printline(terminal_io, "\n");
                            match setting::settings_loop(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            print_user_loop_instructions(terminal_io, credentials);
                            break;
                        }
                        'u' => {
                            printline(terminal_io, "\n");
                            match return_purchase(terminal_io, credentials) {
                                TimeoutResult::TIMEOUT => break 'main,
                                _ => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'q' => {
                            // Legacy behavior wanted by some old users, need not to show in the list of commands
                            break 'main; // Logout
                        }
                        'c' => {
                            // Clear current terminal view
                            // Useful after registering, if you want to see the list of commands
                            // after logging in
                            break print_user_loop_banner(terminal_io, credentials);
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
                            printline(terminal_io, "");
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
                    command = barcode.trim().to_string();
                    utils::printline(terminal_io, "\r\n");
                    if command.is_empty() {
                        break 'main; // Logout
                    } else if command == "exit" {
                        disable_raw_mode().unwrap();
                        exit(0);
                    } else if Regex::new("^[0-9]+$").expect("").is_match(&command) {
                        purchase_items(terminal_io, &command, 1, credentials);
                        printline(terminal_io, "");
                        break;
                    } else {
                        utils::print_error_line(
                            terminal_io,
                            &format!("unknown command: {}\r\n", &command),
                        );
                        break;
                    }
                }
                Ok(InputEvent::Rfid(_)) => {
                    // Logout
                    return;
                }
                _ => (),
            };
        }
    }
}
