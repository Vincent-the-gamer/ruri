/**
 * useToolMessages — Natural-language tool progress messages
 *
 * Instead of cold "🔧 正在使用 read_file..." messages, this composable
 * provides warm, human-like phrases that make the assistant feel like
 * it's naturally telling the user what it's doing.
 *
 * The persona's `tool_response_style` field controls the tone:
 *   - "friendly" (default): Warm and helpful
 *   - "casual": Relaxed, like chatting with a friend
 *   - "professional": Clean and formal
 *   - "cute": Extra expressive, anime-style
 *   - "minimal": Current behavior (tool name only)
 */

export type ToolMessageStyle = 'friendly' | 'casual' | 'professional' | 'cute' | 'minimal'

interface ToolMessageTemplates {
  label: string
  /** Friendly style — warm and helpful */
  friendly: string[]
  /** Casual style — relaxed, like a friend */
  casual: string[]
  /** Professional style — clean, formal */
  professional: string[]
  /** Cute style — extra expressive, anime */
  cute: string[]
  /** Completion messages — shown after tool finishes successfully */
  done: string[]
  /** Failure messages — shown after tool fails */
  failed: string[]
}

// ── Tool name → message templates ──────────────────────────────────
// Each tool has multiple phrase variants per style; we pick one randomly
// so the interaction feels natural rather than repetitive.

const zhToolMessages: Record<string, ToolMessageTemplates> = {
  read_file: {
    label: '读取文件',
    friendly: [
      '让我看看这个文件里有什么... 📖',
      '正在帮你读取文件内容...',
      '我来看一下文件内容 👀',
      '让我瞧瞧里面写了什么...',
    ],
    casual: [
      '瞅一眼这个文件... 👀',
      '看看写了啥...',
      '瞄一下代码~',
    ],
    professional: [
      '正在读取文件内容...',
      '读取文件中...',
    ],
    cute: [
      '让我康康这个文件里有什么！📖✨',
      '正在认真读取中... (´• ω •`)',
      '等下哦～让我看看写了什么～♪',
    ],
    done: [
      '文件读取完成 ✅',
      '已读取文件内容 📖',
      '看完了～',
    ],
    failed: [
      '读取文件失败 ❌',
      '文件读取出错了 😞',
      '没能读到文件内容...',
    ],
  },
  write_file: {
    label: '写入文件',
    friendly: [
      '正在帮你写入文件... ✍️',
      '好的，我来写这个文件...',
      '正在把内容写进去...',
    ],
    casual: [
      '好的，写进去... ✍️',
      '正在写文件~',
    ],
    professional: [
      '正在写入文件...',
      '写入文件中...',
    ],
    cute: [
      '好的！我来帮你写好它～ ✍️✨',
      '正在认真写字中... (｀・ω・´)',
    ],
    done: [
      '文件写入完成 ✅',
      '已写好文件 ✍️',
      '写好了～',
    ],
    failed: [
      '文件写入失败 ❌',
      '写入出错了 😞',
      '没能写入文件...',
    ],
  },
  edit_file: {
    label: '编辑文件',
    friendly: [
      '正在帮你修改文件... ✏️',
      '让我帮你编辑一下...',
      '好的，我来改一下这个文件...',
    ],
    casual: [
      '改一下这个文件... ✏️',
      '修一修~',
    ],
    professional: [
      '正在编辑文件...',
      '编辑文件中...',
    ],
    cute: [
      '让我帮你改改～ ✏️💫',
      '正在小心修改中... (・ω・)',
    ],
    done: [
      '文件编辑完成 ✅',
      '已修改文件 ✏️',
      '改好了～',
    ],
    failed: [
      '编辑文件失败 ❌',
      '修改出错了 😞',
      '没能修改文件...',
    ],
  },
  create_file: {
    label: '创建文件',
    friendly: [
      '正在帮你创建新文件... 📄',
      '好的，新建一个文件...',
      '让我创建一个新文件...',
    ],
    casual: [
      '新建一个文件... 📄',
      '创建中~',
    ],
    professional: [
      '正在创建文件...',
      '创建文件中...',
    ],
    cute: [
      '好哒！新建一个文件～ 📄✨',
      '正在创建新文件呢～♪',
    ],
    done: [
      '文件创建完成 ✅',
      '已创建文件 📄',
      '建好了～',
    ],
    failed: [
      '创建文件失败 ❌',
      '创建出错了 😞',
      '没能创建文件...',
    ],
  },
  delete_file: {
    label: '删除文件',
    friendly: [
      '正在帮你清理文件... 🗑️',
      '好的，我来删除这个文件...',
    ],
    casual: [
      '删掉它... 🗑️',
      '清理一下~',
    ],
    professional: [
      '正在删除文件...',
      '删除文件中...',
    ],
    cute: [
      '好的，清理一下～ 🗑️✨',
      '正在打扫中... (◕‿◕)',
    ],
    done: [
      '文件已删除 ✅',
      '清理完成 🗑️',
      '删好了～',
    ],
    failed: [
      '删除文件失败 ❌',
      '删除出错了 😞',
      '没能删除文件...',
    ],
  },
  list_directory: {
    label: '浏览目录',
    friendly: [
      '让我看看这个目录里有什么... 📂',
      '正在浏览目录内容...',
      '我来看看目录结构...',
    ],
    casual: [
      '看看目录里有啥... 📂',
      '瞄一眼目录~',
    ],
    professional: [
      '正在列出目录内容...',
      '浏览目录中...',
    ],
    cute: [
      '让我康康目录里有啥！📂✨',
      '正在探索目录中... (´▽`)',
    ],
    done: [
      '目录浏览完成 ✅',
      '已列出目录内容 📂',
      '看完了～',
    ],
    failed: [
      '目录浏览失败 ❌',
      '列出目录出错了 😞',
      '没能浏览目录...',
    ],
  },
  search_files: {
    label: '搜索文件',
    friendly: [
      '正在帮你搜索文件... 🔍',
      '让我找找看...',
      '正在查找匹配的文件...',
    ],
    casual: [
      '找找看... 🔍',
      '搜一下~',
    ],
    professional: [
      '正在搜索文件...',
      '搜索文件中...',
    ],
    cute: [
      '让我帮你找找！🔍✨',
      '搜索中～看看能不能找到～♪',
    ],
    done: [
      '搜索完成 ✅',
      '已找到匹配文件 🔍',
      '找好了～',
    ],
    failed: [
      '搜索失败 ❌',
      '搜索出错了 😞',
      '没能搜索到结果...',
    ],
  },
  grep: {
    label: '搜索代码',
    friendly: [
      '正在搜索代码... 🔎',
      '让我在代码里找找...',
      '正在代码库里搜索...',
    ],
    casual: [
      '搜一下代码... 🔎',
      '在代码里翻翻~',
    ],
    professional: [
      '正在搜索代码库...',
      '代码搜索中...',
    ],
    cute: [
      '正在代码里寻宝！🔎✨',
      '翻翻代码～看看有什么～♪',
    ],
    done: [
      '代码搜索完成 ✅',
      '已搜索代码库 🔎',
      '找好了～',
    ],
    failed: [
      '代码搜索失败 ❌',
      '搜索出错了 😞',
      '没能搜索到匹配...',
    ],
  },
  find_path: {
    label: '查找路径',
    friendly: [
      '正在查找文件路径... 🔍',
      '让我找一下这个文件在哪...',
    ],
    casual: [
      '找找文件在哪... 🔍',
    ],
    professional: [
      '正在查找文件路径...',
    ],
    cute: [
      '让我找找文件藏在哪里！🔍✨',
    ],
    done: [
      '路径查找完成 ✅',
      '已找到文件路径 🔍',
      '找到了～',
    ],
    failed: [
      '路径查找失败 ❌',
      '查找出错了 😞',
      '没能找到文件...',
    ],
  },
  bash: {
    label: '执行命令',
    friendly: [
      '正在执行命令，稍等一下... ⚙️',
      '好的，帮你运行这个命令...',
      '正在处理中...',
    ],
    casual: [
      '运行一下... ⚙️',
      '执行命令中~',
    ],
    professional: [
      '正在执行命令...',
      '命令执行中...',
    ],
    cute: [
      '好的！正在努力工作中... ⚙️💪',
      '等一下下哦～正在执行命令～♪',
    ],
    done: [
      '命令执行完成 ✅',
      '已执行完毕 ⚙️',
      '运行好了～',
    ],
    failed: [
      '命令执行失败 ❌',
      '执行出错了 😞',
      '命令没能成功运行...',
    ],
  },
  web_search: {
    label: '搜索网络',
    friendly: [
      '正在帮你搜索相关资料... 🌐',
      '让我上网查查...',
      '正在搜索网络...',
    ],
    casual: [
      '上网搜一下... 🌐',
      '搜搜看~',
    ],
    professional: [
      '正在搜索网络...',
      '网络搜索中...',
    ],
    cute: [
      '正在网上帮你找资料！🌐✨',
      '搜索中～看看能找到什么～♪',
    ],
    done: [
      '网络搜索完成 ✅',
      '已找到相关资料 🌐',
      '搜好了～',
    ],
    failed: [
      '网络搜索失败 ❌',
      '搜索出错了 😞',
      '没能搜到结果...',
    ],
  },
  web_fetch: {
    label: '获取网页',
    friendly: [
      '正在获取网页内容... 🌍',
      '让我看看这个网页...',
      '正在读取网页...',
    ],
    casual: [
      '看看网页... 🌍',
      '打开网页~',
    ],
    professional: [
      '正在获取网页内容...',
      '网页获取中...',
    ],
    cute: [
      '正在打开网页看看！🌍✨',
      '让我看看网页上有什么～♪',
    ],
    done: [
      '网页获取完成 ✅',
      '已获取网页内容 🌍',
      '看好了～',
    ],
    failed: [
      '网页获取失败 ❌',
      '获取网页出错了 😞',
      '没能获取到网页...',
    ],
  },
  invoke_skill: {
    label: '调用技能',
    friendly: [
      '正在调用技能... 🎯',
      '让我用这个技能来处理...',
    ],
    casual: [
      '用个技能... 🎯',
    ],
    professional: [
      '正在调用技能...',
    ],
    cute: [
      '发动技能！🎯✨',
      '正在使用技能中～ (｀・ω・´)',
    ],
    done: [
      '技能执行完成 ✅',
      '技能已执行 🎯',
      '好了～',
    ],
    failed: [
      '技能执行失败 ❌',
      '技能出错了 😞',
      '技能没能成功执行...',
    ],
  },
  fetch: {
    label: '获取内容',
    friendly: [
      '正在获取内容... 📥',
      '让我去拿一下数据...',
    ],
    casual: [
      '拿一下数据... 📥',
    ],
    professional: [
      '正在获取远程内容...',
    ],
    cute: [
      '去拿一下数据～ 📥✨',
    ],
    done: [
      '内容获取完成 ✅',
      '已获取内容 📥',
      '拿到了～',
    ],
    failed: [
      '内容获取失败 ❌',
      '获取出错了 😞',
      '没能获取到内容...',
    ],
  },
  copy_path: {
    label: '复制文件',
    friendly: [
      '正在复制文件... 📋',
      '让我帮你复制一份...',
    ],
    casual: [
      '复制一下... 📋',
    ],
    professional: [
      '正在复制文件...',
    ],
    cute: [
      '复制一份～ 📋✨',
    ],
    done: [
      '文件复制完成 ✅',
      '已复制文件 📋',
      '复制好了～',
    ],
    failed: [
      '文件复制失败 ❌',
      '复制出错了 😞',
      '没能复制文件...',
    ],
  },
  move_path: {
    label: '移动文件',
    friendly: [
      '正在移动文件... 📦',
      '让我帮你挪一下...',
    ],
    casual: [
      '挪一下文件... 📦',
    ],
    professional: [
      '正在移动文件...',
    ],
    cute: [
      '搬家中～ 📦✨',
    ],
    done: [
      '文件移动完成 ✅',
      '已移动文件 📦',
      '搬好了～',
    ],
    failed: [
      '文件移动失败 ❌',
      '移动出错了 😞',
      '没能移动文件...',
    ],
  },
  create_directory: {
    label: '创建目录',
    friendly: [
      '正在创建目录... 📁',
      '让我建个文件夹...',
    ],
    casual: [
      '建个文件夹... 📁',
    ],
    professional: [
      '正在创建目录...',
    ],
    cute: [
      '新建一个文件夹！📁✨',
    ],
    done: [
      '目录创建完成 ✅',
      '已创建目录 📁',
      '建好了～',
    ],
    failed: [
      '目录创建失败 ❌',
      '创建出错了 😞',
      '没能创建目录...',
    ],
  },
}

const enToolMessages: Record<string, ToolMessageTemplates> = {
  read_file: {
    label: 'Reading file',
    friendly: [
      "Let me take a look at this file... 📖",
      "Reading the file for you...",
      "Let me see what's in there... 👀",
    ],
    casual: [
      "Let me peek at this file... 👀",
      "Checking what's inside...",
    ],
    professional: [
      'Reading file...',
      'File read in progress...',
    ],
    cute: [
      "Let me see what's in here! 📖✨",
      "Reading carefully... (´• ω •`)",
    ],
    done: [
      'Done reading the file ✅',
      'File read complete 📖',
      'Got it!',
    ],
    failed: [
      'Failed to read the file ❌',
      'File read error 😞',
      "Couldn't read the file...",
    ],
  },
  write_file: {
    label: 'Writing file',
    friendly: [
      'Writing to the file... ✍️',
      "Let me write this for you...",
    ],
    casual: ['Writing it... ✍️'],
    professional: ['Writing file...'],
    cute: ['Writing it nicely for you! ✍️✨'],
    done: [
      'File written ✅',
      'Done writing ✍️',
      'Written!',
    ],
    failed: [
      'Failed to write file ❌',
      'Write error 😞',
      "Couldn't write the file...",
    ],
  },
  edit_file: {
    label: 'Editing file',
    friendly: [
      'Editing the file for you... ✏️',
      "Let me make those changes...",
    ],
    casual: ['Editing... ✏️'],
    professional: ['Editing file...'],
    cute: ['Making edits carefully~ ✏️💫'],
    done: [
      'File edited ✅',
      'Edits applied ✏️',
      'Done editing!',
    ],
    failed: [
      'Failed to edit file ❌',
      'Edit error 😞',
      "Couldn't edit the file...",
    ],
  },
  create_file: {
    label: 'Creating file',
    friendly: [
      'Creating a new file... 📄',
      "Let me create that file...",
    ],
    casual: ['New file coming up... 📄'],
    professional: ['Creating file...'],
    cute: ['Creating a fresh new file! 📄✨'],
    done: [
      'File created ✅',
      'New file ready 📄',
      'Created!',
    ],
    failed: [
      'Failed to create file ❌',
      'Create error 😞',
      "Couldn't create the file...",
    ],
  },
  delete_file: {
    label: 'Deleting file',
    friendly: [
      'Cleaning up that file... 🗑️',
      "Let me delete this for you...",
    ],
    casual: ['Deleting... 🗑️'],
    professional: ['Deleting file...'],
    cute: ['Tidying up~ 🗑️✨'],
    done: [
      'File deleted ✅',
      'Cleaned up 🗑️',
      'Gone!',
    ],
    failed: [
      'Failed to delete file ❌',
      'Delete error 😞',
      "Couldn't delete the file...",
    ],
  },
  list_directory: {
    label: 'Listing directory',
    friendly: [
      "Let me see what's in this directory... 📂",
      'Browsing the directory...',
    ],
    casual: ["What's in here... 📂"],
    professional: ['Listing directory contents...'],
    cute: ["Exploring the directory! 📂✨"],
    done: [
      'Directory listed ✅',
      'Contents ready 📂',
      'Done!',
    ],
    failed: [
      'Failed to list directory ❌',
      'List error 😞',
      "Couldn't browse the directory...",
    ],
  },
  search_files: {
    label: 'Searching files',
    friendly: [
      'Searching for files... 🔍',
      "Let me find that for you...",
    ],
    casual: ['Looking for it... 🔍'],
    professional: ['Searching files...'],
    cute: ['Hunting for files! 🔍✨'],
    done: [
      'Search complete ✅',
      'Found matching files 🔍',
      'Found them!',
    ],
    failed: [
      'Search failed ❌',
      'Search error 😞',
      "Couldn't find anything...",
    ],
  },
  grep: {
    label: 'Searching code',
    friendly: [
      'Searching through the code... 🔎',
      "Let me find that in the codebase...",
    ],
    casual: ['Searching code... 🔎'],
    professional: ['Searching codebase...'],
    cute: ['Code treasure hunting! 🔎✨'],
    done: [
      'Code search complete ✅',
      'Searched the codebase 🔎',
      'Found!',
    ],
    failed: [
      'Code search failed ❌',
      'Search error 😞',
      "Couldn't find matches...",
    ],
  },
  find_path: {
    label: 'Finding path',
    friendly: [
      "Let me find where that file is... 🔍",
      'Looking up the file path...',
    ],
    casual: ['Where is it... 🔍'],
    professional: ['Finding file path...'],
    cute: ["Where's that file hiding? 🔍✨"],
    done: [
      'Path found ✅',
      'Located the file 🔍',
      'Found it!',
    ],
    failed: [
      'Failed to find path ❌',
      'Path lookup error 😞',
      "Couldn't locate the file...",
    ],
  },
  bash: {
    label: 'Running command',
    friendly: [
      'Running the command, one moment... ⚙️',
      "Let me run that for you...",
    ],
    casual: ['Running it... ⚙️'],
    professional: ['Executing command...'],
    cute: ['Working hard! ⚙️💪'],
    done: [
      'Command finished ✅',
      'Execution complete ⚙️',
      'Done running!',
    ],
    failed: [
      'Command failed ❌',
      'Execution error 😞',
      "The command didn't succeed...",
    ],
  },
  web_search: {
    label: 'Searching web',
    friendly: [
      'Searching the web for you... 🌐',
      "Let me look that up...",
    ],
    casual: ['Googling... 🌐'],
    professional: ['Searching web...'],
    cute: ['Searching the internet! 🌐✨'],
    done: [
      'Web search complete ✅',
      'Found results 🌐',
      'Got the results!',
    ],
    failed: [
      'Web search failed ❌',
      'Search error 😞',
      "Couldn't find results...",
    ],
  },
  web_fetch: {
    label: 'Fetching page',
    friendly: [
      'Fetching the webpage... 🌍',
      "Let me get that page...",
    ],
    casual: ['Getting the page... 🌍'],
    professional: ['Fetching webpage...'],
    cute: ['Opening the webpage! 🌍✨'],
    done: [
      'Page fetched ✅',
      'Webpage ready 🌍',
      'Got the page!',
    ],
    failed: [
      'Failed to fetch page ❌',
      'Fetch error 😞',
      "Couldn't get the page...",
    ],
  },
  invoke_skill: {
    label: 'Invoking skill',
    friendly: [
      'Using a skill to handle this... 🎯',
      "Let me apply a skill...",
    ],
    casual: ['Using a skill... 🎯'],
    professional: ['Invoking skill...'],
    cute: ['Activating skill! 🎯✨'],
    done: [
      'Skill executed ✅',
      'Skill complete 🎯',
      'Done!',
    ],
    failed: [
      'Skill failed ❌',
      'Skill error 😞',
      "The skill didn't complete...",
    ],
  },
  fetch: {
    label: 'Fetching',
    friendly: [
      'Fetching content... 📥',
      "Let me grab that data...",
    ],
    casual: ['Getting data... 📥'],
    professional: ['Fetching remote content...'],
    cute: ['Fetching data~ 📥✨'],
    done: [
      'Content fetched ✅',
      'Data ready 📥',
      'Got it!',
    ],
    failed: [
      'Failed to fetch ❌',
      'Fetch error 😞',
      "Couldn't get the data...",
    ],
  },
  copy_path: {
    label: 'Copying file',
    friendly: [
      'Copying the file... 📋',
      "Let me make a copy...",
    ],
    casual: ['Copying... 📋'],
    professional: ['Copying file...'],
    cute: ['Making a copy! 📋✨'],
    done: [
      'File copied ✅',
      'Copy ready 📋',
      'Copied!',
    ],
    failed: [
      'Failed to copy file ❌',
      'Copy error 😞',
      "Couldn't copy the file...",
    ],
  },
  move_path: {
    label: 'Moving file',
    friendly: [
      'Moving the file... 📦',
      "Let me relocate this...",
    ],
    casual: ['Moving it... 📦'],
    professional: ['Moving file...'],
    cute: ['Moving things around~ 📦✨'],
    done: [
      'File moved ✅',
      'Relocated 📦',
      'Moved!',
    ],
    failed: [
      'Failed to move file ❌',
      'Move error 😞',
      "Couldn't move the file...",
    ],
  },
  create_directory: {
    label: 'Creating directory',
    friendly: [
      'Creating a directory... 📁',
      "Let me make a folder...",
    ],
    casual: ['Making a folder... 📁'],
    professional: ['Creating directory...'],
    cute: ['Making a new folder! 📁✨'],
    done: [
      'Directory created ✅',
      'Folder ready 📁',
      'Created!',
    ],
    failed: [
      'Failed to create directory ❌',
      'Create error 😞',
      "Couldn't create the directory...",
    ],
  },
}

// ── Helper: pick a random variant ──────────────────────────────────

function pickRandom(arr: string[]): string {
  return arr[Math.floor(Math.random() * arr.length)] ?? arr[0]
}

// ── Main export ────────────────────────────────────────────────────

/**
 * Generate a friendly, human-like progress message for a tool being executed.
 */
export function getToolProgressMessage(
  toolName: string,
  locale: string,
  style: ToolMessageStyle = 'friendly',
): string {
  const messages = locale === 'zh-CN' ? zhToolMessages : enToolMessages

  // If the style is 'minimal', return the old compact format
  if (style === 'minimal') {
    const label = messages[toolName]?.label ?? toolName.replace(/_/g, ' ')
    return locale === 'zh-CN'
      ? `> 🔧 正在使用 \`${label}\`...`
      : `> 🔧 Using \`${label}\`...`
  }

  const tmpl = messages[toolName]
  if (!tmpl) {
    // Unknown tool — use a friendly generic message
    const generic: Record<string, string[]> = {
      friendly: locale === 'zh-CN'
        ? ['正在处理，请稍候... 💭', '好的，正在处理中...', '让我来处理这个... 🤔']
        : ['Working on it... 💭', 'Let me handle this...', 'Processing... 🤔'],
      casual: locale === 'zh-CN'
        ? ['处理中... 💭', '等一下下~']
        : ['Working on it... 💭', 'One sec~'],
      professional: locale === 'zh-CN'
        ? ['正在处理...', '处理中...']
        : ['Processing...', 'Working...'],
      cute: locale === 'zh-CN'
        ? ['正在处理中～等一下下哦 (´▽`)', '好的呢～正在努力工作中！💭✨']
        : ['Working on it~ (´▽`)', 'Hang tight! 💭✨'],
    }
    return pickRandom(generic[style] ?? generic.friendly)
  }

  const variants = tmpl[style]
  if (!variants || variants.length === 0) {
    // Fallback to friendly
    return pickRandom(tmpl.friendly)
  }

  return pickRandom(variants)
}

/**
 * Generate a tool progress message that includes argument context.
 * e.g. "让我看看 src/main.rs 里写了什么... 📖"
 */
export function getToolProgressMessageWithArgs(
  toolName: string,
  argsPreview: string,
  locale: string,
  style: ToolMessageStyle = 'friendly',
): string {
  const base = getToolProgressMessage(toolName, locale, style)

  // For minimal style, append args preview like before
  if (style === 'minimal') {
    if (argsPreview) {
      const shortArgs = argsPreview.length > 80 ? argsPreview.slice(0, 80) + '...' : argsPreview
      return base.replace('...', ` with \`${shortArgs}\`...`)
    }
    return base
  }

  // For human-like styles, don't append raw args — the message is already
  // natural enough. The args are visible in the collapsible tool details.
  return base
}

/**
 * Generate a completion message when a tool finishes executing.
 * Shows success ✅ or failure ❌ so the user knows the result.
 */
export function getToolCompletionMessage(
  toolName: string,
  ok: boolean,
  locale: string,
): string {
  const messages = locale === 'zh-CN' ? zhToolMessages : enToolMessages
  const tmpl = messages[toolName]
  if (!tmpl) {
    // Unknown tool — use generic completion messages
    return ok
      ? (locale === 'zh-CN' ? '处理完成 ✅' : 'Done ✅')
      : (locale === 'zh-CN' ? '处理失败 ❌' : 'Failed ❌')
  }
  const variants = ok ? tmpl.done : tmpl.failed
  if (!variants || variants.length === 0) {
    return ok
      ? (locale === 'zh-CN' ? '处理完成 ✅' : 'Done ✅')
      : (locale === 'zh-CN' ? '处理失败 ❌' : 'Failed ❌')
  }
  return pickRandom(variants)
}
