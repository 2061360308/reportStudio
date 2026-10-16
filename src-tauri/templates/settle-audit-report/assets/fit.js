// 自适应字号：对 class="fit" 的单元格，按 data-max 字数在超出时按比例调小字号。
(function () {
  function fitCell(td) {
    var max = parseInt(td.getAttribute("data-max"), 10) || 0;
    var text = (td.textContent || "").trim();
    var base = 13.5,
      min = 9;
    if (max > 0 && text.length > max) {
      td.style.fontSize =
        Math.max(min, (base * max) / text.length).toFixed(2) + "pt";
    }
  }
  function run() {
    var cells = document.querySelectorAll("td.fit");
    for (var i = 0; i < cells.length; i++) fitCell(cells[i]);
  }
  if (document.readyState === "loading")
    document.addEventListener("DOMContentLoaded", run);
  else run();
  window.addEventListener("load", run);
})();
