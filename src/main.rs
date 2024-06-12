//! Implements the TUI interface

use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use log::{debug, info, trace, error};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, path::PathBuf};
use std::{env, io, process::Command};

use budget_tracker::expense::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Add entry
    #[arg(short, long)]
    add: bool,

    /// Edit entries
    #[arg(short, long)]
    edit: bool,

    /// Check logs
    #[arg(short, long)]
    logs: bool,

    /// Search entries
    #[arg(short, long)]
    search: Option<String>,
}

fn get_expenses_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Unable to determine user's home directory")?;
    Ok(home_dir.join(".local").join("share").join("budget-tracker"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{:?} {} {}] {}",
                Utc::now(),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(fern::log_file(get_expenses_dir()?.join("expenses.log"))?)
        .apply()?;
    info!("====Starting program====");
    let args = Args::parse();

    if args.add {
        Expense::add_expense()?;
        trace!("Added the expense succesfully");
    }

    if args.edit {
        Expense::edit_expenses("expenses.csv")?;
        trace!("Edited file succesfully");
    }

    if args.logs {
        trace!("Opening the log file ...");
        Command::new("tail")
            .arg("-f")
            .arg(get_expenses_dir()?.join("expenses.log").to_str().unwrap())
            .status()?;
        trace!("Closed log file view succesfully");
        invoke_gracefull_exit()?;
    }

    trace!("Starting the TUI ...");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    trace!("Reading expenses.csv ...");
    let mut expenses = match Expense::read_csv("expenses.csv") {
        Ok(expenses) => expenses,
        Err(err) => {
            error!("Error reading CSV, trying to create it: {}", err);
            match Expense::create_expenses_csv() {
                Ok(_) => Vec::new(),
                Err(err) => {
                    error!("Error creating CSV: {}", err);
                    return Err(err);
                }
            }
        }
    };

    if let Some(query) = &args.search {
        trace!("Found user query: {}", query);
        let matcher = SkimMatcherV2::default();
        expenses = expenses
            .iter()
            .filter(|expense| {
                matcher.fuzzy_match(&expense.description, query).is_some()
                    || matcher
                        .fuzzy_match(&expense.expense_type.to_string(), query)
                        .is_some()
            })
            .cloned()
            .collect();
    }

    // Sort expenses by date in descending order
    expenses.sort_by(|a, b| b.date.cmp(&a.date));

    let mut should_quit = false;
    let mut table_state = TableState::default().with_selected(Some(0));
    let table_size = expenses.len();
    while !should_quit {
        terminal.draw(|f| ui(f, &expenses, &mut table_state))?;
        should_quit = handle_events(&mut table_state, table_size)?;
    }
    
    invoke_gracefull_exit()?;
    Ok(())
}

fn invoke_gracefull_exit() -> Result<(), Box<dyn std::error::Error>>{
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    info!("====Exiting the program====");
    std::process::exit(0);

    Ok(())
}

fn handle_events(table_state: &mut TableState, table_size: usize) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            ..
        }) = event::read()?
        {
            debug!("Read in key: {:?}", code);
            match code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Down | KeyCode::Char('s') => {
                    if let Some(selected) = table_state.selected() {
                        let next_index = if selected >= table_size - 1 {
                            0
                        } else {
                            selected + 1
                        };
                        table_state.select(Some(next_index));
                    }
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    if let Some(selected) = table_state.selected() {
                        let next_index = if selected == 0 {
                            table_size - 1
                        } else {
                            selected - 1
                        };
                        table_state.select(Some(next_index));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, expenses: &[Expense], table_state: &mut TableState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(frame.size());

    // Split the second chunk (chunks[1]) vertically into two equal parts
    let charts_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let positive_chunk = charts_chunks[0];
    let negative_chunk = charts_chunks[1];

    // Calculate the total sum of amounts
    let total_amount: f64 = expenses.iter().map(|expense| expense.amount).sum();
    let total_spent: f64 = expenses
        .iter()
        .filter(|expense| expense.amount < 0.0)
        .map(|expense| expense.amount)
        .sum();
    let total_earned: f64 = expenses
        .iter()
        .filter(|expense| expense.amount >= 0.0)
        .map(|expense| expense.amount)
        .sum();

    // Expense Table
    let rows = expenses
        .iter()
        .map(|expense| {
            Row::new(vec![
                expense.date.clone(),
                expense.description.clone(),
                capitalize(expense.expense_type.to_string()),
                expense.amount.to_string(),
            ])
        })
        .collect::<Vec<Row>>();

    let widths = [
        Constraint::Length(15),
        Constraint::Length(65),
        Constraint::Length(20),
        Constraint::Length(10),
    ];

    let expense_table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL))
        .header(
            Row::new(vec!["Date", "Description", "Type", "Amount"]).style(Style::default().bold()),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>");

    let table_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(chunks[0]);

    // frame.render_widget(expense_table, chunks[0]);
    frame.render_stateful_widget(expense_table, table_chunks[0], table_state);

    let rows = vec![
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Net Total Spent".to_string(),
            total_amount.to_string(),
        ])
        .style(Style::default().bold())
        .top_margin(1),
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Total Spent".to_string(),
            total_spent.to_string(),
        ])
        .style(Style::default().bold()),
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Total Earned".to_string(),
            total_earned.to_string(),
        ])
        .style(Style::default().bold()),
    ];

    let data_table = Table::new(rows, widths);

    frame.render_widget(data_table, table_chunks[1]);

    // Aggregate expenses by date
    let mut aggregated_expenses: HashMap<String, f64> = HashMap::new();
    for expense in expenses {
        let entry = aggregated_expenses
            .entry(expense.expense_type.to_string())
            .or_insert(0.0);
        *entry += expense.amount;
    }

    // Separate positive and negative expenses
    let total_earned_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount >= 0.0)
        .collect();

    let total_spent_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount < 0.0)
        .map(|(expense_type, amount)| (capitalize(expense_type), -amount))
        .collect();

    for (mut expense_data, chunk, title, color) in [
        (
            total_spent_data.clone(),
            positive_chunk,
            "Expenditure",
            Style::default().cyan(),
        ),
        (
            total_earned_data,
            negative_chunk,
            "Income",
            Style::default().red(),
        ),
    ] {
        expense_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Find the maximum expense amount
        let max_expense_amount = expense_data
            .iter()
            .map(|(_, amount)| *amount)
            .fold(f64::NEG_INFINITY, f64::max);

        // Convert type expenses to bar chart data
        let type_data: Vec<(&str, u64)> = expense_data
            .iter()
            .map(|(date, amount)| (date.as_str(), *amount as u64))
            .collect();

        // Calculate dynamic bar width
        let available_width = chunk.width as usize;
        let num_types = expense_data.len() + 5;
        let min_bar_width = 1;

        let bar_width = if num_types > 0 {
            (available_width / num_types).max(min_bar_width) as u16
        } else {
            min_bar_width as u16
        };

        let type_barchart = BarChart::default()
            .block(Block::default().title(title).borders(Borders::ALL))
            .bar_width(bar_width)
            // .bar_gap(1)
            // .group_gap(3)
            .bar_style(color)
            .value_style(Style::default().white().bold())
            .label_style(Style::default().white())
            .data(&type_data)
            .max(max_expense_amount.ceil() as u64);

        frame.render_widget(type_barchart, chunk); // Render the type barchart
    }
}
