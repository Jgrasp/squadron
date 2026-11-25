export default {
  tab: 'MCP servers',
  intro: 'Servers from your Claude Code user configuration. On demand, a server starts in no session: an agent starts it for itself with /mcp, or on its own when its task needs it.',
  connectors:
    "claude.ai connectors on demand are turned off with Claude Code's own switch, in each folder where an agent starts, so they stay off there in a terminal too: turn one back on with /mcp. Squadron never touches a connector you turned off yourself, and gives back the ones it turned off when you set them to Every session, at the next agent start in that folder.",
  onDemand: 'On demand',
  everywhere: 'Every session',
  description: 'What it is for, so that agents know when to start it',
  none: "No server in Claude Code's user configuration.",
  hintOff: 'on demand · off',
  hintOn: 'on demand · on',
  start: 'Start for this agent',
  stop: 'Stop for this agent',
  started: '{server} starts for this agent.',
  stopped: '{server} is stopped for this agent.',
}
