import React from "https://esm.sh/react@19";

export const pages = {
  "observatory.events.page": function ObservatoryPage(props) {
    return React.createElement(
      "div",
      {
        style: {
          padding: "16px",
          borderRadius: "12px",
          border: "1px solid rgba(255,255,255,0.12)",
          background: "rgba(255,255,255,0.04)",
        },
      },
      [
        React.createElement("h2", { key: "title" }, "Observatory Runtime Page"),
        React.createElement(
          "p",
          { key: "ext" },
          `extension=${props.extension?.id ?? "unknown"} source=${props.extension?.source_mode ?? "unknown"}`,
        ),
        React.createElement(
          "p",
          { key: "page" },
          `page=${props.page?.id ?? "unknown"} mount=${props.page?.mount ?? "unknown"}`,
        ),
      ],
    );
  },
};

export default pages["observatory.events.page"];
