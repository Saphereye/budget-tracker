/// Defines all [Expense] struct related objects.

use chrono::{Local, NaiveDate};
use std::io::{self, BufRead, BufReader, Write};
use std::{env, process::Command};
use std::{fs, path::PathBuf};

// expense.rs
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ExpenseType {
    Food,
    Travel,
    Fun,
    Medical,
    Personal,
    Other(String), // To handle custom expense types
}

impl fmt::Display for ExpenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpenseType::Food => write!(f, "Food"),
            ExpenseType::Travel => write!(f, "Travel"),
            ExpenseType::Fun => write!(f, "Fun"),
            ExpenseType::Medical => write!(f, "Medical"),
            ExpenseType::Personal => write!(f, "Personal"),
            ExpenseType::Other(custom) => write!(f, "{}", custom),
        }
    }
}

impl std::str::FromStr for ExpenseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "food" => Ok(ExpenseType::Food),
            "travel" => Ok(ExpenseType::Travel),
            "fun" => Ok(ExpenseType::Fun),
            "medical" => Ok(ExpenseType::Medical),
            "personal" => Ok(ExpenseType::Personal),
            other => Ok(ExpenseType::Other(other.to_string())),
        }
    }
}

/// The `Expense` struct; helps reading/writing data in a structured manner. It reflects the schema of the database.
#[derive(Debug, Clone)]
pub struct Expense {
    pub date: String,
    pub description: String,
    pub expense_type: ExpenseType,
    pub amount: f64,
}

impl Expense {
    pub fn new(date: String, description: String, expense_type: ExpenseType, amount: f64) -> Self {
        Self {
            date,
            description,
            expense_type,
            amount,
        }
    }

    /**
    Function to add and expense to the database.

    Takes input from `stdin` for date, description, expense type and amount.
    Support YYYY-MM-DD and YYYY/MM/DD date format as input.
    For amount no denoination is expected as of now.
    */
    pub fn add_expense() -> Result<(), Box<dyn std::error::Error>> {
        let date = Self::input_date()?;
        let description = Self::input("Enter description:")?;
        let expense_type = Self::input_expense_type()?;
        let amount = Self::input_amount()?;
        let expense = Self::new(date, description, expense_type, amount);

        Self::append_to_csv("expenses.csv", &expense)?;
        println!("Added your data to the db!");

        Ok(())
    }

    fn input(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut input = String::new();
        print!("{}", prompt);
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn input_date() -> Result<String, Box<dyn std::error::Error>> {
        loop {
            let input = Self::input(
                "Enter date (YYYY-MM-DD or YYYY/MM/DD, leave empty for today's date): ",
            )?;
            if input.is_empty() {
                return Ok(Local::now().format("%Y-%m-%d").to_string());
            } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
                return Ok(date.to_string());
            } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y/%m/%d") {
                return Ok(date.to_string());
            } else {
                println!("Invalid date format. Please enter the date in YYYY-MM-DD or YYYY/MM/DD format.");
            }
        }
    }

    fn input_expense_type() -> Result<ExpenseType, Box<dyn std::error::Error>> {
        loop {
            let input = Self::input(
                "Enter expense type (Food, Travel, Fun, Medical, Personal or Other): ",
            )?;
            match input.parse() {
                Ok(expense_type) => return Ok(expense_type),
                Err(_) => println!("Invalid expense type. Please try again."),
            }
        }
    }

    fn input_amount() -> Result<f64, Box<dyn std::error::Error>> {
        loop {
            let input = Self::input("Enter amount: ")?;
            match input.trim().parse() {
                Ok(amount) => return Ok(amount),
                Err(_) => println!("Invalid amount. Please enter a valid number."),
            }
        }
    }

    /// Allows editing the database by specifying an EDITOR environment variable. By default its nano.
    pub fn edit_expenses(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        Command::new(editor)
            .arg(Expense::get_database_file_path(file_name)?)
            .status()?;

        Ok(())
    }

    /// Allows adding data to the end of the database
    pub fn append_to_csv(
        file_name: &str,
        expense: &Expense,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Expense::get_database_file_path(file_name)?;
        let mut file = fs::OpenOptions::new().append(true).open(file_path)?;
        let data = format!(
            "{},{},{},{}\n",
            expense.date, expense.description, expense.expense_type, expense.amount
        );
        file.write_all(data.as_bytes())?;

        Ok(())
    }

    /// Read the database if its present from ~/.local/share/budget-tracker/expenses.csv;
    /// if not present it returns an error.
    pub fn read_csv(file_name: &str) -> Result<Vec<Expense>, Box<dyn std::error::Error>> {
        let file_path = Expense::get_database_file_path(file_name)?;
        let file = fs::File::open(file_path)?;

        let reader = BufReader::new(file);
        let mut expenses = Vec::new();

        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if index == 0 {
                continue; // Skip header
            }
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() == 4 {
                let expense_type: ExpenseType = fields[2].parse()?;
                let expense = Expense::new(
                    fields[0].to_string(),
                    fields[1].to_string(),
                    expense_type,
                    fields[3].parse::<f64>()?,
                );
                expenses.push(expense);
            }
        }
        Ok(expenses)
    }

    /// Creates the database. Usually called when running the program for the first time.
    pub fn create_expenses_csv() -> Result<(), Box<dyn std::error::Error>> {
        let budget_tracker_dir = Expense::get_database_file_path("")?;
        if let Err(err) = fs::create_dir_all(&budget_tracker_dir) {
            eprintln!(
                "Error creating directory {}: {}",
                budget_tracker_dir.display(),
                err
            );
            return Err(err.into());
        }

        let expenses_file = budget_tracker_dir.join("expenses.csv");
        if let Err(err) = fs::File::create(&expenses_file) {
            eprintln!("Error creating file {}: {}", expenses_file.display(), err);
            return Err(err.into());
        }
        Ok(())
    }

    fn get_database_file_path(file_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = dirs::home_dir().ok_or("Unable to determine user's home directory")?;
        Ok(home_dir
            .join(".local")
            .join("share")
            .join("budget-tracker")
            .join(file_name))
    }
}
