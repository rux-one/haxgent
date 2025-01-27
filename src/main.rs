mod logger;
mod tools;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{collections::HashMap, io, thread::current};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use chrono::{DateTime, Utc, TimeZone};
use crossbeam_channel::{unbounded, Sender, Receiver};

use crate::logger::Logger;
use crate::tools::{Tool, SystemCommandTool, ChatTool, ToolResult};

#[derive(Debug, Clone, PartialEq)]
enum AgentMessage {
    Poke,
    SetHost(String),
}

#[derive(Debug, Clone, PartialEq)]
enum AgentState {
    Idle,
    Scanning,
}

#[derive(Debug, Clone)]
struct AgentConfig {
    host: String,
}

#[derive(Debug, Clone)]
struct Agent {
    config: AgentConfig,
    state: AgentState,
    log_sender: Sender<(String, String)>,
    scan_tool: SystemCommandTool,
    chat_tool: ChatTool,
}

impl Agent {
    fn new(log_sender: Sender<(String, String)>) -> Self {
        Agent {
            config: AgentConfig {
                host: String::from("127.0.0.1"),
            },
            state: AgentState::Idle,
            log_sender,
            scan_tool: SystemCommandTool::new(
                "Rustscan".to_string(),
                "Network scanning tool".to_string(),
                "rustscan".to_string(),
            ),
            chat_tool: ChatTool::new(
                "ChatGPT".to_string(),
                "AI assistant for analyzing scan results".to_string(),
            ),
        }
    }

    fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }

    fn get_state(&self) -> &AgentState {
        &self.state
    }

    fn set_host(&mut self, host: String) {
        self.config.host = host;
    }

    fn get_host(&self) -> &String {
        &self.config.host
    }

    fn poke(&mut self) {
        if self.state == AgentState::Scanning {
            return;
        }

        self.set_state(AgentState::Scanning);
        self.log_sender.send((
            String::from("Starting scan... ⏳"),
            format!("Scanning host {} with rustscan", self.config.host)
        )).unwrap();

        // Run nmap scan
        let args = vec![
            "-a".to_string(),
            self.config.host.clone(),
            "-r".to_string(),
            "0-10000".to_string(),
            "--".to_string(),
            "-sVCT".to_string(),
            "-oX".to_string(),
            "nmap_report.xml".to_string(),
        ];

        match self.scan_tool.run(args) {
            Ok(ToolResult::Success(_)) => {
                self.log_sender.send((
                    String::from("Scan completed successfully ☑️"),
                    "Scan results are saved to `nmap_report.xml`".to_string(),
                )).unwrap();
                
                // Ask LLM to summarize the findings

                self.log_sender.send((
                    String::from("I am looking at `nmap_report.xml` file... 👓"),
                    String::from("Analyzing scan results...\n\nReading through the `nmap_report.xml` file.")
                )).unwrap();

                if let Ok(scan_data) = std::fs::read_to_string("nmap_report.xml") {
                    match self.chat_tool.run(vec![format!(
                        "Please analyze this nmap scan result and provide security insights: {}",
                        scan_data
                    )]) {
                        Ok(ToolResult::Success(analysis)) => {
                            self.log_sender.send((
                                String::from("I have something for you... 📄"),
                                analysis
                            )).unwrap();
                        }
                        Ok(ToolResult::Error(err)) => {
                            self.log_sender.send((
                                String::from("Forgive me for I have failed (1) ⛔"),
                                err
                            )).unwrap();
                        }
                        Err(e) => {
                            self.log_sender.send((
                                String::from("Forgive me for I have failed (2) ⛔"),
                                e.to_string()
                            )).unwrap();
                        }
                    }
                }
            }
            Ok(ToolResult::Error(err)) => {
                self.log_sender.send((
                    String::from("Scan failed"),
                    err
                )).unwrap();
            }
            Err(e) => {
                self.log_sender.send((
                    String::from("Error during scan"),
                    e.to_string()
                )).unwrap();
            }
        }

        self.set_state(AgentState::Idle);
    }

    fn handle_message(&mut self, msg: AgentMessage) {
        match msg {
            AgentMessage::Poke => self.poke(),
            AgentMessage::SetHost(host) => {
                self.set_host(host);
                self.log_sender.send((
                    format!("Now looking 🔍 at host: {}", self.config.host),
                    format!("Host changed to {}", self.get_host())
                )).unwrap();
                self.poke();
            }
        }
    }
}

struct Commands {
    commands: HashMap<String, String>,
    current_command: String,
    current_command_args: Vec<String>,
}

impl Commands {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
            current_command: String::new(),
            current_command_args: vec![],
        }
    }

    fn add_command(&mut self, command: String, description: String) {
        self.commands.insert(command, description);
    }

    fn setup_commands(&self) -> HashMap<String, String> {
        let mut commands = HashMap::new();
        commands.insert("sethost".to_string(), "Set host".to_string());
        commands.insert("help".to_string(), "Available commands: hello, quit, sethost".to_string());
        commands.insert("quit".to_string(), "Goodbye!".to_string());
        commands.insert("poke".to_string(), "Poke the agent".to_string());
        commands
    }

    fn capture_command(&mut self, input: &str) {
        if input.is_empty() {
            return;
        }

        let cmd = input.split(" ").collect::<Vec<&str>>()[0];
        let args = input.split(" ").collect::<Vec<&str>>()[1..].join(" ");

        self.current_command = String::from(cmd);
        self.current_command_args = args.split(" ").map(|s| s.to_string()).collect();
    }

    fn get_current_command(&self) -> &String {
        &self.current_command
    }

    fn get_current_command_args(&self) -> &Vec<String> {
        &self.current_command_args
    }

    fn reset_command(&mut self) {
        self.current_command = String::new();
        self.current_command_args = vec![];
    }
}

struct App {
    input: String,
    agent: Agent,
    commands: Commands,
    log: Logger,
    agent_sender: Sender<AgentMessage>,
    log_receiver: Receiver<(String, String)>,
}

impl App {
    fn new() -> Self {
        let (agent_sender, agent_receiver) = unbounded();
        let (log_sender, log_receiver) = unbounded();
        
        let mut app = App {
            input: String::new(),
            agent: Agent::new(log_sender.clone()),
            commands: Commands::new(),
            log: Logger::new(),
            agent_sender,
            log_receiver,
        };

        // Spawn a thread to handle agent messages
        std::thread::spawn({
            let mut agent = app.agent.clone();
            let receiver = agent_receiver;
            move || {
                while let Ok(msg) = receiver.recv() {
                    agent.handle_message(msg);
                }
            }
        });

        app
    }

    fn next_log(&mut self) {
        let logs = self.log.get_logs();
        if logs.is_empty() {
            return;
        }
        let i = match self.log.get_selected() {
            Some(i) => {
                if i >= logs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.log.select(Some(i));
    }

    fn previous_log(&mut self) {
        let logs = self.log.get_logs();
        if logs.is_empty() {
            return;
        }
        let i = match self.log.get_selected() {
            Some(i) => {
                if i == 0 {
                    logs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.log.select(Some(i));
    }
}

fn main() -> Result<(), io::Error> {
    let mut app = App::new();

    app.log.add(
        String::from("Welcome! 👋🏼"),
        String::from("The default host has been set to 127.0.0.1"),
    );

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    terminal.clear()?;
    
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .constraints([Constraint::Max(3), Constraint::Max(5), Constraint::Max(1)].as_ref())
                .split(size);

            app.commands.capture_command(&app.input);

            let logs = app.log.get_logs();
            let log_items: Vec<ListItem> = logs
                .iter()
                .map(|entry| {
                    let dt = Utc.timestamp_opt(entry.created_at as i64, 0).unwrap();
                    let content = format!("{} -- {}", dt.format("%Y-%m-%d %H:%M:%S"), entry.summary);
                    ListItem::new(content)
                })
                .collect();

            let log_list = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title(" -- log --"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            let mut list_state = ListState::default();
            list_state.select(app.log.get_selected());

            let details_content = if let Some(selected) = app.log.get_selected() {
                if let Some(entry) = logs.get(selected) {
                    format!("Time: {}\nSummary: {}\n\nDetails:\n{}", 
                        Utc.timestamp_opt(entry.created_at as i64, 0).unwrap().format("%Y-%m-%d %H:%M:%S"),
                        entry.summary,
                        entry.details)
                } else {
                    String::from("No log selected")
                }
            } else {
                String::from("No log selected")
            };

            let log_details_widget = Paragraph::new(details_content)
                .block(Block::default().borders(Borders::ALL).title(" -- details --"));
            
            let log_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(chunks[2]);

            f.render_stateful_widget(log_list, log_chunks[0], &mut list_state);
            f.render_widget(log_details_widget, log_chunks[1]);

            let settings = format!("Target host: {}", app.agent.get_host());
            let settings_widget = Paragraph::new(settings)
                .block(Block::default().borders(Borders::ALL).title(" -- settings -- "));
            f.render_widget(settings_widget, chunks[1]);
            
            let input_widget = Paragraph::new(app.input.as_ref() as &str)
                .block(Block::default().borders(Borders::ALL).title(" -- command --"));
            f.render_widget(input_widget, chunks[0]);
        })?;

        // Check for any new log messages
        while let Ok((summary, details)) = app.log_receiver.try_recv() {
            app.log.add(summary, details);
        }

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {

                match key.code {
                    crossterm::event::KeyCode::Enter => {
                        // Execute command on Enter key
                        match app.commands.get_current_command().as_str() {
                            "sethost" => {
                                let host = app.commands.get_current_command_args()[0].clone();
                                app.agent_sender.send(AgentMessage::SetHost(host)).unwrap();
                            },
                            "poke" => {
                                app.agent_sender.send(AgentMessage::Poke).unwrap();
                            },
                            "exit" => break,
                            _ => {}
                        }

                        app.input.clear();  // Clear input after execution
                    }
                    crossterm::event::KeyCode::Esc => break,  // Exit on Esc key
                    crossterm::event::KeyCode::Char(c) => {
                        app.input.push(c);  // Add character to input
                    }
                    crossterm::event::KeyCode::Backspace => {
                        app.input.pop();  // Remove last character on Backspace
                    }
                    crossterm::event::KeyCode::Up => app.previous_log(),
                    crossterm::event::KeyCode::Down => app.next_log(),
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
