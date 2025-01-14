import { OpenAiChatService } from "./chat/openai-chat.service.ts";
const API_KEY = Deno.env.get("OPENAI_API_KEY");

const chatService = new OpenAiChatService(API_KEY!);

async function readFile(filePath: string): Promise<string> {
  try {
    const fileContent = await Deno.readTextFile(filePath);
    return fileContent;
  } catch (error) {
    throw new Error(`Error reading file: ${error.message}`);
  }
}

async function writeFile(filePath: string, content: string): Promise<void> {
  try {
    await Deno.writeTextFile(filePath, content);
  } catch (error) {
    throw new Error(`Error writing file: ${error.message}`);
  }
}

async function runCommand(command: string, args: string[] = []): Promise<{ stdout: string; stderr: string }> {
  const process = new Deno.Command(command, {
    args,
    stdout: "piped",
    stderr: "piped",
  });

  const { stdout, stderr } = await process.output();

  return {
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
  };
}

const tools = {
  'scan': async (host: string) => runCommand("rustscan", ["-a", host, "-r", "0-10000", "--", "-sCVT", "-oX", "nmap_report.xml"]),
}

const { stdout, stderr } = await tools['scan']("127.0.0.1");
if (stderr) {
  console.error("stderr:", stderr);
}

chatService.setSystemMessage(`
  You are a cybersecurity expert. 
  Your focus is reconnaissance.
  You will receive an Nmap XML report.
  Your task is to analyze the report and provide a summary of the findings.
  The summary will be concise and to the point. 
  The summary will me in markdown format.
  Bullet points are preferred.
`)

const xmlReport = await readFile("./nmap_report.xml");
const summary = await chatService.sendMessage(xmlReport, 'user');

console.log(summary);
await writeFile("./summary.md", summary);

export {}