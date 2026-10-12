#!/usr/bin/env node
const readline = require('readline');
const { execSync } = require('child_process');

const BOLD = '\x1b[1m';
const RESET = '\x1b[0m';
const CYAN = '\x1b[36m';
const GREEN = '\x1b[32m';
const YELLOW = '\x1b[33m';
const GRAY = '\x1b[90m';
const MAGENTA = '\x1b[35m';

function clearScreen() {
  process.stdout.write('\x1b[2J\x1b[3J\x1b[H');
}

function openUrl(url) {
  try {
    const start = process.platform === 'darwin' ? 'open' : process.platform === 'win32' ? 'start' : 'xdg-open';
    execSync(`${start} "${url}"`, { stdio: 'ignore' });
    return true;
  } catch (e) {
    return false;
  }
}

function renderPopup() {
  clearScreen();
  console.log('\n');
  console.log(`  ${CYAN}╭────────────────────────────────────────────────────────────────────────╮${RESET}`);
  console.log(`  ${CYAN}│${RESET}   ${BOLD}${MAGENTA}★ VANTA HAS MOVED TO ORBIT ★${RESET}                                         ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}├────────────────────────────────────────────────────────────────────────┤${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  Vanta is now officially rebranded and superseded by ${BOLD}${GREEN}Orbit${RESET}.            ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  Orbit brings faster Ratatui rendering, Extism WASM extensions,        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  and community themes in a single terminal pane.                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  ${BOLD}${YELLOW}MIGRATION INSTRUCTIONS:${RESET}                                               ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}    ${GRAY}1. Delete old Vanta package:${RESET}                                         ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}       ${BOLD}npm uninstall -g @ziuus/vanta${RESET}                                     ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}    ${GRAY}2. Install Orbit:${RESET}                                                    ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}       ${BOLD}${GREEN}npm install -g @ziuus/orbit${RESET}                                       ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}       ${GRAY}or cargo install --git https://github.com/ziuus/orbit-tui${RESET}        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  ${BOLD}LINKS:${RESET}                                                                  ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  • ${CYAN}GitHub:${RESET} https://github.com/ziuus/orbit-tui                              ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}  • ${CYAN}npm:${RESET}    https://www.npmjs.com/package/@ziuus/orbit                     ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}│${RESET}                                                                        ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}├────────────────────────────────────────────────────────────────────────┤${RESET}`);
  console.log(`  ${CYAN}│${RESET}  ${BOLD}[Enter]${RESET} Open Orbit GitHub in browser   ${BOLD}[O]${RESET} Open npm package   ${BOLD}[q]${RESET} Exit    ${CYAN}│${RESET}`);
  console.log(`  ${CYAN}╰────────────────────────────────────────────────────────────────────────╯${RESET}`);
  console.log('\n');
}

renderPopup();

if (process.stdin.isTTY) {
  readline.emitKeypressEvents(process.stdin);
  process.stdin.setRawMode(true);

  process.stdin.on('keypress', (str, key) => {
    if (key.name === 'q' || (key.ctrl && key.name === 'c') || key.name === 'escape') {
      process.stdin.setRawMode(false);
      process.exit(0);
    } else if (key.name === 'return') {
      openUrl('https://github.com/ziuus/orbit-tui');
      console.log(`  ${GREEN}✓ Opened GitHub: https://github.com/ziuus/orbit-tui${RESET}\n`);
      setTimeout(() => {
        process.stdin.setRawMode(false);
        process.exit(0);
      }, 700);
    } else if (str === 'o' || str === 'O') {
      openUrl('https://www.npmjs.com/package/@ziuus/orbit');
      console.log(`  ${GREEN}✓ Opened npm: https://www.npmjs.com/package/@ziuus/orbit${RESET}\n`);
      setTimeout(() => {
        process.stdin.setRawMode(false);
        process.exit(0);
      }, 700);
    }
  });
} else {
  // Non-interactive fallback
  process.exit(0);
}
