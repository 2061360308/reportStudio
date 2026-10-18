(function () {
  "use strict";

  var DEFAULT_RELEASES = "https://github.com/2061360308/reportStudio/releases";
  var cachedIndex = null;
  var cachedClient = null;
  var indexPromise = null;
  var active = null;

  function releasesUrl() {
    var repo = window.$docsify && window.$docsify.repo;
    if (typeof repo === "string" && repo.indexOf("/") > 0) {
      return "https://github.com/" + repo + "/releases";
    }
    return DEFAULT_RELEASES;
  }

  function normalizeOs(value) {
    var text = String(value || "").toLowerCase();
    if (text.indexOf("windows") >= 0 || text === "win32") return "windows";
    if (
      text.indexOf("mac") >= 0 ||
      text.indexOf("darwin") >= 0 ||
      text.indexOf("os x") >= 0
    ) {
      return "darwin";
    }
    if (text.indexOf("linux") >= 0 || text.indexOf("x11") >= 0) return "linux";
    return "unknown";
  }

  function normalizeArch(value, bitness) {
    var text = String(value || "").toLowerCase();
    if (!text) return "";
    if (text.indexOf("arm64") >= 0 || text.indexOf("aarch64") >= 0 || text === "arm") {
      return "aarch64";
    }
    if (text.indexOf("ia32") >= 0 || text.indexOf("i686") >= 0) return "i686";
    if (
      text.indexOf("x86_64") >= 0 ||
      text.indexOf("amd64") >= 0 ||
      text === "x86" ||
      text.indexOf("x64") >= 0
    ) {
      return String(bitness) === "32" ? "i686" : "x86_64";
    }
    return "";
  }

  function archFromUserAgent(userAgent) {
    if (/arm64|aarch64/i.test(userAgent)) return "aarch64";
    if (/i686|i386|win32/i.test(userAgent) && !/x64|win64|wow64/i.test(userAgent)) {
      return "i686";
    }
    return "x86_64";
  }

  function detectClient() {
    var userAgent = navigator.userAgent || "";
    var fallback = {
      os: normalizeOs(navigator.platform || userAgent),
      arch: archFromUserAgent(userAgent),
    };

    if (
      navigator.userAgentData &&
      typeof navigator.userAgentData.getHighEntropyValues === "function"
    ) {
      return navigator.userAgentData
        .getHighEntropyValues(["architecture", "bitness", "platform"])
        .then(function (data) {
          return {
            os:
              normalizeOs(data.platform) !== "unknown"
                ? normalizeOs(data.platform)
                : fallback.os,
            arch: normalizeArch(data.architecture, data.bitness) || fallback.arch,
          };
        })
        .catch(function () {
          return fallback;
        });
    }

    return Promise.resolve(fallback);
  }

  function normalizePlatforms(platforms) {
    var result = [];
    var seen = {};
    (Array.isArray(platforms) ? platforms : []).forEach(function (platform) {
      if (!platform || !platform.href) return;
      var key = String(platform.key || "");
      var os = normalizeOs(platform.os || key.split("-")[0]);
      var arch = normalizeArch(platform.arch || key.split("-")[1]) || String(platform.arch || "");
      if (!os || !arch) return;
      var id = os + "-" + arch;
      if (seen[id]) return;
      seen[id] = true;
      result.push({
        key: key,
        label: platform.label || platformLabel(os, arch),
        os: os,
        arch: arch,
        href: platform.href,
      });
    });
    return result;
  }

  function platformLabel(os, arch) {
    var osLabels = {
      windows: "Windows",
      darwin: "macOS",
      linux: "Linux",
    };
    var archLabels = {
      x86_64: "x64",
      aarch64: "ARM64",
      i686: "32-bit",
    };
    return [osLabels[os] || os, archLabels[arch] || arch].filter(Boolean).join(" ");
  }

  function selectPlatform(platforms, client) {
    if (!platforms.length) return null;

    var exact = platforms.filter(function (platform) {
      return platform.os === client.os && platform.arch === client.arch;
    })[0];
    if (exact) return exact;

    var sameOs = platforms.filter(function (platform) {
      return platform.os === client.os;
    });
    if (sameOs.length) {
      return (
        sameOs.filter(function (platform) {
          return platform.arch === "x86_64";
        })[0] || sameOs[0]
      );
    }

    var windows = platforms.filter(function (platform) {
      return platform.os === "windows";
    });
    return (
      windows.filter(function (platform) {
        return platform.arch === "x86_64";
      })[0] ||
      windows[0] ||
      platforms[0]
    );
  }

  function resolveHref(href) {
    try {
      return new URL(href, new URL(".", window.location.href)).href;
    } catch (error) {
      return href;
    }
  }

  function fetchIndex() {
    if (indexPromise) return indexPromise;
    var url = new URL("downloads/index.json", new URL(".", window.location.href));
    indexPromise = fetch(url.href, { cache: "no-store" })
      .then(function (response) {
        if (!response.ok) throw new Error("HTTP " + response.status);
        return response.json();
      })
      .then(function (data) {
        if (!data || !Array.isArray(data.platforms)) {
          throw new Error("下载索引格式错误");
        }
        cachedIndex = data;
        return data;
      })
      .catch(function (error) {
        indexPromise = null;
        throw error;
      });
    return indexPromise;
  }

  function makeElement(tag, className, text) {
    var element = document.createElement(tag);
    if (className) element.className = className;
    if (text !== undefined) element.textContent = text;
    return element;
  }

  function setOpen(control, open) {
    control.open = open;
    control.host.dataset.open = open ? "true" : "false";
    control.toggle.setAttribute("aria-expanded", open ? "true" : "false");
    control.menu.hidden = !open;
  }

  function openMenu(control, focusFirst) {
    if (!control || control.host.dataset.state === "loading") return;
    setOpen(control, true);
    if (focusFirst) {
      var first = control.menu.querySelector("a");
      if (first) first.focus();
    }
  }

  function addMenuItem(control, options) {
    var link = makeElement("a");
    link.href = options.href;
    link.setAttribute("role", "menuitem");

    var title = makeElement("span", "cover-download__title", options.title);
    if (options.badge) {
      title.appendChild(makeElement("span", "cover-download__badge", options.badge));
    }
    link.appendChild(title);

    if (options.meta) {
      link.appendChild(makeElement("span", "cover-download__meta", options.meta));
    }
    if (options.external) {
      link.target = "_blank";
      link.rel = "noreferrer noopener";
    }
    link.addEventListener("click", function () {
      setOpen(control, false);
    });
    control.menu.appendChild(link);
  }

  function renderMenu(control, index, platforms, selected, client) {
    var menu = control.menu;
    var releases = resolveHref(index.releases || releasesUrl());
    menu.innerHTML = "";

    var archRank = function (arch) {
      if (arch === "x86_64") return 0;
      if (arch === "aarch64") return 1;
      return 2;
    };
    var sameOs = platforms
      .filter(function (platform) {
        return platform.os === selected.os;
      })
      .sort(function (a, b) {
        return archRank(a.arch) - archRank(b.arch) || a.label.localeCompare(b.label);
      });

    sameOs.forEach(function (platform) {
      var isCurrent = platform.os === client.os && platform.arch === client.arch;
      addMenuItem(control, {
        href: resolveHref(platform.href),
        title: platform.label,
        meta: index.version ? "v" + index.version : "",
        badge: isCurrent ? "当前系统" : "",
      });
    });

    if (sameOs.length) {
      menu.appendChild(makeElement("div", "cover-download__divider"));
    }

    addMenuItem(control, {
      href: releases,
      title: "更多版本",
      meta: "在 GitHub Releases 查看全部版本",
      external: true,
    });
  }

  function applyIndex(control, index, client) {
    var platforms = normalizePlatforms(index.platforms);
    var selected = selectPlatform(platforms, client);
    var releases = resolveHref(index.releases || releasesUrl());

    control.host.dataset.state = "ready";

    if (!selected) {
      control.main.href = releases;
      control.main.textContent = "下载最新版本";
      control.main.title = "下载适配当前系统的最新版本";
      control.main.setAttribute("aria-label", "下载适配当前系统的最新版本");
      control.menu.innerHTML = "";
      addMenuItem(control, {
        href: releases,
        title: "更多版本",
        meta: "在 GitHub Releases 查看全部版本",
        external: true,
      });
      return;
    }

    var version = index.version ? " v" + index.version : "";
    control.main.href = resolveHref(selected.href);
    control.main.textContent = "下载 " + selected.label + version;
    control.main.title = "下载适配当前系统的最新版本";
    control.main.setAttribute(
      "aria-label",
      "下载 " + selected.label + version + "（当前系统）",
    );

    renderMenu(control, index, platforms, selected, client);
  }

  function showFallback(control) {
    var releases = releasesUrl();
    control.host.dataset.state = "fallback";
    control.main.href = releases;
    control.main.textContent = "下载最新版本";
    control.main.title = "下载适配当前系统的最新版本";
    control.main.setAttribute("aria-label", "下载适配当前系统的最新版本");
    control.menu.innerHTML = "";
    addMenuItem(control, {
      href: releases,
      title: "更多版本",
      meta: "在 GitHub Releases 查看全部版本",
      external: true,
    });
  }

  function createControl(host) {
    host.innerHTML = "";

    var main = makeElement("a", "cover-download__main", "正在获取最新版本…");
    main.href = releasesUrl();
    main.setAttribute("aria-label", "正在获取最新版本下载地址");

    var toggle = makeElement("button", "cover-download__toggle");
    toggle.type = "button";
    toggle.setAttribute("aria-label", "展开更多下载选项");
    toggle.setAttribute("aria-haspopup", "true");
    toggle.setAttribute("aria-expanded", "false");
    toggle.appendChild(makeElement("span", "cover-download__caret"));

    var menu = makeElement("div", "cover-download__menu");
    menu.hidden = true;
    menu.setAttribute("role", "menu");

    host.appendChild(main);
    host.appendChild(toggle);
    host.appendChild(menu);

    var control = {
      host: host,
      main: main,
      toggle: toggle,
      menu: menu,
      open: false,
    };

    toggle.addEventListener("click", function (event) {
      event.stopPropagation();
      setOpen(control, !control.open);
    });

    toggle.addEventListener("keydown", function (event) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        openMenu(control, true);
      }
    });

    return control;
  }

  function mount() {
    var host = document.getElementById("cover-download");
    if (!host) return;
    if (host.dataset.bound === "true") return;

    host.dataset.bound = "true";
    var control = createControl(host);
    active = control;

    if (cachedIndex && cachedClient) {
      applyIndex(control, cachedIndex, cachedClient);
      return;
    }

    Promise.all([fetchIndex(), cachedClient ? Promise.resolve(cachedClient) : detectClient()])
      .then(function (results) {
        cachedClient = results[1];
        applyIndex(control, results[0], cachedClient);
      })
      .catch(function (error) {
        console.warn("[cover-download] 无法读取下载索引，回退到 GitHub Releases", error);
        showFallback(control);
      });
  }

  document.addEventListener("click", function (event) {
    if (active && active.open && !active.host.contains(event.target)) {
      setOpen(active, false);
    }
  });

  document.addEventListener("keydown", function (event) {
    if (!active || !active.open) return;
    if (event.key === "Escape") {
      setOpen(active, false);
      active.toggle.focus();
    }
  });

  window.addEventListener("load", mount);
  window.ReportStudioDownload = { mount: mount };
})();
