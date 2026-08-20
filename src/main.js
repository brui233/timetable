const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

let schedule = { headers: [], rows: [] };
let config = { start_date: null };
let currentWeek = null;

const gridEl = document.getElementById("grid");
const weekInfoEl = document.getElementById("week-info");

// 解析类似 "(1-12, 14-17周)" 的周数信息，返回上课周次集合；未写明则返回 null（表示每周都上）
function parseActiveWeeks(text) {
  const match = text.match(/[（(](.*?周)[）)]/);
  if (!match) return null;
  const weekStr = match[1];
  const active = new Set();
  const regex = /(\d+)(?:-(\d+))?/g;
  let m;
  while ((m = regex.exec(weekStr)) !== null) {
    const p1 = parseInt(m[1], 10);
    const p2 = m[2] ? parseInt(m[2], 10) : null;
    if (p2 !== null) {
      for (let i = p1; i <= p2; i++) active.add(i);
    } else {
      active.add(p1);
    }
  }
  return active;
}

function computeCurrentWeek() {
  currentWeek = null;
  let suffix = "";
  if (config.start_date) {
    const start = new Date(config.start_date + "T00:00:00");
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const diffDays = Math.floor((today - start) / (1000 * 60 * 60 * 24));
    if (diffDays >= 0) {
      currentWeek = Math.floor(diffDays / 7) + 1;
      suffix = `当前：第 ${currentWeek} 周`;
    } else {
      suffix = "尚未开学";
    }
  } else {
    suffix = "未设置开学日期";
  }
  weekInfoEl.textContent = suffix;
}

function todayWeekdayIndex() {
  // JS: 0=周日..6=周六 -> 转成 0=周一..6=周日，方便和表头列对齐（第0列是"节次"）
  const jsDay = new Date().getDay();
  return jsDay === 0 ? 6 : jsDay - 1;
}

function render() {
  computeCurrentWeek();
  const todayCol = todayWeekdayIndex(); // 对应 headers 下标 = todayCol + 1

  gridEl.innerHTML = "";
  const colCount = schedule.headers.length;
  gridEl.style.gridTemplateColumns = `80px repeat(${colCount - 1}, 1fr)`;

  // 表头
  schedule.headers.forEach((h, colIdx) => {
    const div = document.createElement("div");
    div.className = "cell header";
    if (colIdx - 1 === todayCol) div.classList.add("today");
    div.textContent = h;
    div.contentEditable = "true";
    div.addEventListener("blur", () => {
      schedule.headers[colIdx] = div.textContent.trim();
      saveSchedule();
    });
    gridEl.appendChild(div);
  });

  // 内容行
  schedule.rows.forEach((row, rowIdx) => {
    for (let colIdx = 0; colIdx < colCount; colIdx++) {
      const raw = (row[colIdx] ?? "").toString();
      const div = document.createElement("div");
      div.className = "cell editable";
      div.contentEditable = "true";

      if (colIdx === 0) {
        div.classList.add("period");
        div.textContent = raw;
      } else {
        const isToday = colIdx - 1 === todayCol;
        if (isToday) div.classList.add("today");
        if (raw.trim()) div.classList.add("has-content");

        let displayText = raw;
        if (raw.trim() && currentWeek !== null) {
          const activeWeeks = parseActiveWeeks(raw);
          if (activeWeeks !== null && !activeWeeks.has(currentWeek)) {
            div.classList.add("not-this-week");
            displayText = `[非本周]\n${raw}`;
          }
        }
        div.textContent = displayText;
      }

      div.addEventListener("focus", () => {
        // 编辑时去掉 [非本周] 提示前缀，只留原始内容，避免误存
        div.textContent = raw;
      });

      div.addEventListener("blur", () => {
        const newVal = div.textContent.trim();
        if (!schedule.rows[rowIdx]) schedule.rows[rowIdx] = [];
        schedule.rows[rowIdx][colIdx] = newVal;
        saveSchedule();
        render();
      });

      gridEl.appendChild(div);
    }
  });
}

async function loadAll() {
  try {
    schedule = await invoke("load_schedule");
    config = await invoke("load_config");
  } catch (e) {
    console.error(e);
    alert("加载数据失败: " + e);
  }
  render();
}

async function saveSchedule() {
  try {
    await invoke("save_schedule", { data: schedule });
  } catch (e) {
    console.error(e);
  }
}

async function saveConfig() {
  try {
    await invoke("save_config", { config });
  } catch (e) {
    console.error(e);
  }
}

document.getElementById("btn-import").addEventListener("click", async () => {
  const filePath = await open({
    multiple: false,
    filters: [
      { name: "课表文件", extensions: ["xlsx", "xls", "csv"] },
    ],
  });
  if (!filePath) return;
  try {
    const data = await invoke("import_file", { path: filePath });
    schedule = data;
    await saveSchedule();
    render();
  } catch (e) {
    alert("导入失败：\n" + e);
  }
});

document.getElementById("btn-set-date").addEventListener("click", async () => {
  const defaultVal = config.start_date || new Date().toISOString().slice(0, 10);
  const input = prompt(
    "请输入开学第一周『星期一』的日期\n格式: YYYY-MM-DD (例如 2024-02-26)",
    defaultVal
  );
  if (!input) return;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(input.trim())) {
    alert("日期格式不正确，请严格按照 YYYY-MM-DD 格式输入！");
    return;
  }
  config.start_date = input.trim();
  await saveConfig();
  render();
});

document.getElementById("btn-add-row").addEventListener("click", () => {
  const blank = new Array(schedule.headers.length).fill("");
  blank[0] = `第${schedule.rows.length + 1}节`;
  schedule.rows.push(blank);
  saveSchedule();
  render();
});

document.getElementById("btn-del-row").addEventListener("click", () => {
  if (schedule.rows.length === 0) return;
  schedule.rows.pop();
  saveSchedule();
  render();
});

loadAll();
