window.__RS_TEMPLATE_VALIDATE__ = function (ctx) {
  var values = ctx.values || {};
  var variables = Array.isArray(ctx.variables) ? ctx.variables : [];
  var issues = [];

  function labelFor(key) {
    var variable = variables.find(function (item) {
      return item.key === key;
    });
    return (
      (variable && (variable.label || variable.alias || variable.key)) || key
    );
  }

  function isEmpty(value) {
    return value === undefined || value === null || String(value).trim() === "";
  }

  function toNumber(value) {
    if (isEmpty(value)) return null;
    var number = Number(value);
    return Number.isFinite(number) ? number : null;
  }

  function isRealDate(value) {
    var match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    if (!match) return false;

    var year = Number(match[1]);
    var month = Number(match[2]);
    var day = Number(match[3]);
    if (month < 1 || month > 12 || day < 1 || day > 31) return false;

    var date = new Date(Date.UTC(year, month - 1, day));
    return (
      date.getUTCFullYear() === year &&
      date.getUTCMonth() === month - 1 &&
      date.getUTCDate() === day
    );
  }

  // 1. 所有可输入的日期变量必须是真实存在的日期（含闰年、月份天数）。
  var dates = {};
  variables
    .filter(function (variable) {
      return variable.type === "date";
    })
    .forEach(function (variable) {
      var key = variable.key;
      var value = values[key];
      if (isEmpty(value)) {
        dates[key] = { present: false, valid: false, value: "" };
        return;
      }

      var text = String(value).trim();
      var valid = isRealDate(text);
      dates[key] = { present: true, valid: valid, value: text };

      if (!valid) {
        issues.push({
          status: "error",
          keys: [key],
          key: key,
          msg: labelFor(key) + "不是有效日期：" + text,
        });
      }
    });

  function compareDates(earlierKey, laterKey, message) {
    var earlier = dates[earlierKey];
    var later = dates[laterKey];
    if (
      earlier &&
      later &&
      earlier.present &&
      later.present &&
      earlier.valid &&
      later.valid &&
      later.value <= earlier.value
    ) {
      issues.push({
        status: "error",
        keys: [earlierKey, laterKey],
        key: laterKey,
        msg: message,
      });
    }
  }

  // 2. 工程验收日期 > 工程竣工日期 > 工程开工日期。
  compareDates(
    "start_date",
    "end_date",
    "工程竣工时间必须晚于工程开工时间",
  );
  compareDates(
    "end_date",
    "accept_date",
    "工程验收时间必须晚于工程竣工时间",
  );

  // 3. 审核计划竣工 > 审核计划开工。
  compareDates(
    "plan_start_date",
    "plan_end_date",
    "审核计划竣工时间必须晚于审核计划开工时间",
  );

  // 4. 报审金额必须大于审定金额。
  var reportedAmount = toNumber(values.reported_amount);
  var approvedAmount = toNumber(values.approved_amount);
  if (
    reportedAmount !== null &&
    approvedAmount !== null &&
    reportedAmount <= approvedAmount
  ) {
    issues.push({
      status: "error",
      keys: ["reported_amount", "approved_amount"],
      key: "reported_amount",
      msg: "报审金额必须大于审定金额",
    });
  }

  // 5. 委托合同书编号：SXHS-年份-编号，年份不超过当前年，编号暂定 1-999。
  var contractNo = isEmpty(values.contract_no)
    ? ""
    : String(values.contract_no).trim();
  if (contractNo) {
    var contractMatch = /^SXHS-(\d{4})-(\d+)$/.exec(contractNo);
    if (!contractMatch) {
      issues.push({
        status: "error",
        keys: ["contract_no"],
        key: "contract_no",
        msg: "委托合同书编号格式应为 SXHS-年份-编号，例如 SXHS-2026-243",
      });
    } else {
      var contractYear = Number(contractMatch[1]);
      var contractSerial = Number(contractMatch[2]);
      var currentYear = new Date().getFullYear();

      if (contractYear > currentYear) {
        issues.push({
          status: "error",
          keys: ["contract_no"],
          key: "contract_no",
          msg: "委托合同书编号年份不能超过当前年份 " + currentYear,
        });
      } else if (contractSerial < 1 || contractSerial > 999) {
        issues.push({
          status: "error",
          keys: ["contract_no"],
          key: "contract_no",
          msg: "委托合同书编号的流水号必须在 1-999 之间",
        });
      }
    }
  }

  // 6. 工程内容超过 20 字只给警告，不阻止导出。
  if (!isEmpty(values.scope)) {
    var scopeText = String(values.scope).trim();
    var scopeLength = Array.from(scopeText).length;
    if (scopeLength > 20) {
      issues.push({
        status: "warning",
        keys: ["scope"],
        key: "scope",
        msg: "工程内容建议不超过 20 字（当前 " + scopeLength + " 字）",
      });
    }
  }

  return issues;
};
