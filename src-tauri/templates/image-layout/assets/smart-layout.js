(function (global) {
  "use strict";

  var PAGE_W = 190;
  var PAGE_H = 277;
  var MIN_DPI = 150;
  var MM_PER_INCH = 25.4;
  var MAX_PER_ROW = 8;
  var MAX_ROWS_PER_PAGE = 8;
  var WEIGHT_STRETCH = 60;
  var WEIGHT_WHITESPACE = 50;
  var WEIGHT_VARIANCE = 20;
  var WEIGHT_LEGIBILITY = 5000;
  var WEIGHT_DPI = 20;
  var WEIGHT_PAGE = 4;
  var DEFAULTS = {
    maxPerPage: 6,
    maxStretch: 0.08,
    minShort: 30,
    maxRowRatio: 0,
    order: "smart",
  };

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function resolveOptions(source) {
    var input = source || {};
    var maxPerPage = parseInt(input.maxPerPage, 10);
    if (!isFinite(maxPerPage)) maxPerPage = DEFAULTS.maxPerPage;
    maxPerPage = clamp(maxPerPage, 1, 12);

    var maxStretch = parseFloat(input.maxStretch);
    if (!isFinite(maxStretch)) maxStretch = DEFAULTS.maxStretch;
    maxStretch = clamp(maxStretch, 0, 0.2);

    var minShort = parseFloat(input.minShort);
    if (!isFinite(minShort)) minShort = DEFAULTS.minShort;
    minShort = clamp(minShort, 20, 80);

    var maxRowRatio = parseFloat(input.maxRowRatio);
    if (!isFinite(maxRowRatio) || maxRowRatio <= 0) {
      maxRowRatio = 0;
    } else {
      maxRowRatio = clamp(maxRowRatio, 1.2, 3);
    }

    return {
      maxPerPage: maxPerPage,
      maxStretch: maxStretch,
      minShort: minShort,
      maxRowRatio: maxRowRatio,
      order: input.order === "original" ? "original" : "smart",
    };
  }

  function normalize(images, options) {
    var list = images.slice();
    if (options.order !== "original") {
      list.sort(function (a, b) {
        return a.ratio - b.ratio || (a.order || 0) - (b.order || 0);
      });
    }
    return list.map(function (item) {
      var pxW = item.pxW || 1200;
      var pxH = item.pxH || 900;
      return {
        ratio: item.ratio || pxW / pxH,
        pxW: pxW,
        pxH: pxH,
        image: item.image || null,
        order: item.order || 0,
      };
    });
  }

  function makeRow(images, start, end) {
    var sum = 0;
    var cells = [];
    for (var i = start; i <= end; i += 1) sum += images[i].ratio;
    var naturalH = PAGE_W / sum;
    var h = Math.min(naturalH, PAGE_H);
    for (var j = start; j <= end; j += 1) {
      var item = images[j];
      cells.push({
        ratio: item.ratio,
        w: item.ratio * h,
        pxW: item.pxW,
        pxH: item.pxH,
        image: item.image,
      });
    }
    return {
      h: h,
      width: sum * h,
      capped: naturalH > PAGE_H,
      cells: cells,
    };
  }

  function pageMetrics(rows, options) {
    var sum = 0;
    for (var i = 0; i < rows.length; i += 1) sum += rows[i].h;
    if (sum <= 0) return null;

    var minLambda = 1 / (1 + options.maxStretch);
    var maxLambda = 1 / (1 - options.maxStretch);
    var lambda = clamp(PAGE_H / sum, minLambda, maxLambda);
    var used = lambda * sum;
    if (used > PAGE_H + 0.001) return null;

    var heights = [];
    var minH = Infinity;
    var maxH = 0;
    for (var h = 0; h < rows.length; h += 1) {
      var rowH = lambda * rows[h].h;
      heights.push(rowH);
      minH = Math.min(minH, rowH);
      maxH = Math.max(maxH, rowH);
    }
    if (
      options.maxRowRatio > 0 &&
      minH > 0 &&
      maxH / minH > options.maxRowRatio + 1e-9
    ) {
      return null;
    }

    var s = 1 / lambda;
    var stretch = Math.abs(s - 1);
    var whitespace = Math.max(0, PAGE_H - used);
    var cost =
      Math.pow(stretch, 2) * WEIGHT_STRETCH +
      Math.pow(whitespace / PAGE_H, 2) * WEIGHT_WHITESPACE +
      WEIGHT_PAGE;

    for (var r = 0; r < rows.length; r += 1) {
      var height = heights[r];
      for (var c = 0; c < rows[r].cells.length; c += 1) {
        var cell = rows[r].cells[c];
        var shortSide = Math.min(cell.w, height);
        if (shortSide < options.minShort) {
          var shortMiss = (options.minShort - shortSide) / options.minShort;
          cost += shortMiss * shortMiss * WEIGHT_LEGIBILITY;
        }
        var dpiX = cell.pxW / (cell.w / MM_PER_INCH);
        var dpiY = cell.pxH / (height / MM_PER_INCH);
        var miss = Math.max(
          dpiX < MIN_DPI ? (MIN_DPI - dpiX) / MIN_DPI : 0,
          dpiY < MIN_DPI ? (MIN_DPI - dpiY) / MIN_DPI : 0,
        );
        if (miss > 0) cost += miss * miss * WEIGHT_DPI;
      }
    }

    if (maxH > 0) {
      var variance = (maxH - minH) / PAGE_H;
      cost += variance * variance * WEIGHT_VARIANCE;
    }

    return {
      lambda: lambda,
      used: used,
      whitespace: whitespace,
      cost: cost,
    };
  }

  function buildPage(images, start, sizes, options) {
    var rows = [];
    var cursor = start;
    for (var i = 0; i < sizes.length; i += 1) {
      rows.push(makeRow(images, cursor, cursor + sizes[i] - 1));
      cursor += sizes[i];
    }
    var metrics = pageMetrics(rows, options);
    if (!metrics) return null;
    return {
      rows: rows,
      lambda: metrics.lambda,
      used: metrics.used,
      whitespace: metrics.whitespace,
      cost: metrics.cost,
    };
  }

  function bestPage(images, start, count, options) {
    var best = null;
    var sizes = [];

    function walk(remaining) {
      if (remaining === 0) {
        var page = buildPage(images, start, sizes, options);
        if (page && (!best || page.cost < best.cost)) best = page;
        return;
      }
      if (sizes.length >= MAX_ROWS_PER_PAGE) return;
      var limit = Math.min(MAX_PER_ROW, remaining);
      for (var size = 1; size <= limit; size += 1) {
        sizes.push(size);
        walk(remaining - size);
        sizes.pop();
      }
    }

    walk(count);
    return best;
  }

  function planLayout(images, sourceOptions) {
    var options = resolveOptions(sourceOptions);
    var list = normalize(images, options);
    var n = list.length;
    if (!n) return [];

    var best = new Array(n + 1);
    var choice = new Array(n + 1);
    best[n] = 0;
    choice[n] = null;

    for (var start = n - 1; start >= 0; start -= 1) {
      best[start] = Infinity;
      choice[start] = null;
      var limit = Math.min(options.maxPerPage, n - start);
      for (var count = 1; count <= limit; count += 1) {
        var page = bestPage(list, start, count, options);
        if (!page) continue;
        var cost = page.cost + best[start + count] - count * 1e-6;
        if (cost < best[start]) {
          best[start] = cost;
          choice[start] = { count: count, page: page };
        }
      }
      if (!choice[start]) {
        var relaxed = resolveOptions({
          maxPerPage: options.maxPerPage,
          maxStretch: options.maxStretch,
          minShort: options.minShort,
          maxRowRatio: 0,
          order: options.order,
        });
        var fallback = buildPage(list, start, [1], relaxed);
        choice[start] = { count: 1, page: fallback };
        best[start] = fallback ? fallback.cost : 0;
      }
    }

    var pages = [];
    var cursor = 0;
    while (cursor < n) {
      var item = choice[cursor];
      if (!item || !item.page) break;
      pages.push(item.page);
      cursor += item.count;
    }
    return pages;
  }

  var api = {
    planLayout: planLayout,
    resolveOptions: resolveOptions,
    constants: {
      PAGE_W: PAGE_W,
      PAGE_H: PAGE_H,
      MIN_DPI: MIN_DPI,
      DEFAULTS: DEFAULTS,
    },
  };

  if (typeof module !== "undefined" && module.exports) module.exports = api;
  if (typeof window === "undefined") return;

  global.__RS_SMART_LAYOUT__ = api;

  function mm(value) {
    return value.toFixed(3) + "mm";
  }

  function collect(root) {
    var nodes = root.querySelectorAll("img");
    var items = [];
    for (var i = 0; i < nodes.length; i += 1) {
      var img = nodes[i];
      var w = img.naturalWidth || 0;
      var h = img.naturalHeight || 0;
      if (!w || !h) {
        w = 1200;
        h = 900;
      }
      items.push({
        ratio: w / h,
        pxW: w,
        pxH: h,
        image: img,
        order: i,
      });
    }
    return items;
  }

  function readOptions(root) {
    return {
      maxPerPage: root.getAttribute("data-max-images"),
      maxStretch: root.getAttribute("data-max-stretch"),
      minShort: root.getAttribute("data-min-short"),
      maxRowRatio: root.getAttribute("data-max-row-ratio"),
      order: root.getAttribute("data-order"),
    };
  }

  function render(root, pages) {
    var fragment = document.createDocumentFragment();

    for (var p = 0; p < pages.length; p += 1) {
      var page = pages[p];
      var section = document.createElement("section");
      section.className = "sheet smart-sheet";
      var canvas = document.createElement("div");
      canvas.className = "smart-canvas";

      for (var r = 0; r < page.rows.length; r += 1) {
        var row = page.rows[r];
        var rowHeight = page.lambda * row.h;
        var rowEl = document.createElement("div");
        rowEl.className = "smart-row";
        rowEl.style.height = mm(rowHeight);

        for (var c = 0; c < row.cells.length; c += 1) {
          var cell = row.cells[c];
          var fig = document.createElement("figure");
          fig.className = "smart-cell";
          fig.style.width = mm(cell.w);
          fig.style.height = mm(rowHeight);
          if (cell.image) {
            cell.image.style.width = "100%";
            cell.image.style.height = "100%";
            cell.image.style.objectFit = "fill";
            cell.image.style.display = "block";
            fig.appendChild(cell.image);
          }
          rowEl.appendChild(fig);
        }
        canvas.appendChild(rowEl);
      }

      section.appendChild(canvas);
      fragment.appendChild(section);
    }

    while (root.firstChild) root.removeChild(root.firstChild);
    root.appendChild(fragment);
  }

  function run() {
    if (global.__rsSmartLayoutDone) return;
    global.__rsSmartLayoutDone = true;
    var root = document.querySelector("[data-smart-layout]");
    if (!root) return;
    try {
      var items = collect(root);
      if (!items.length) return;
      render(root, planLayout(items, readOptions(root)));
    } catch (error) {
      global.__rsSmartLayoutError = String(error);
    }
  }

  if (document.readyState === "complete") run();
  else global.addEventListener("load", run);
})(typeof window !== "undefined" ? window : globalThis);
