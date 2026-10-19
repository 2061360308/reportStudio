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

  // 所有可输入日期必须是真实存在的日期（含闰年、月份天数）。
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

  // 计划竣工日期必须晚于计划开工日期。
  compareDates(
    "plan_start_date",
    "plan_end_date",
    "计划竣工日期必须晚于计划开工日期",
  );

  // 实际竣工日期必须晚于实际开工日期。
  compareDates(
    "actual_start_date",
    "actual_end_date",
    "实际竣工日期必须晚于实际开工日期",
  );

  return issues;
};
