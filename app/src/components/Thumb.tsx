import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { cx } from "./ui";

const cache = new Map<string, string>();

export function Thumb({
  src,
  alt,
  fallback,
  className,
  width = 480,
  eager = false,
}: {
  src: string | null | undefined;
  alt: string;
  fallback?: string;
  className?: string;
  width?: number;
  eager?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [data, setData] = useState("");
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setData("");
    setFailed(false);
    if (!src) return;

    const key = `${src}|${width}`;
    const hit = cache.get(key);
    if (hit) {
      setData(hit);
      return;
    }

    if (eager) {
      let cancelled = false;
      api
        .contentImage(src, width)
        .then((url) => {
          if (cancelled) return;
          cache.set(key, url);
          setData(url);
        })
        .catch(() => {
          if (!cancelled) setFailed(true);
        });
      return () => {
        cancelled = true;
      };
    }

    const node = ref.current;
    if (!node) return;
    let cancelled = false;

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        api
          .contentImage(src, width)
          .then((url) => {
            if (cancelled) return;
            cache.set(key, url);
            setData(url);
          })
          .catch(() => {
            if (!cancelled) setFailed(true);
          });
      },
      { rootMargin: "220px" },
    );
    observer.observe(node);

    return () => {
      cancelled = true;
      observer.disconnect();
    };
  }, [src, width, eager]);

  return (
    <div className={cx("thumb", className)} ref={ref}>
      {data ? (
        <img src={data} alt={alt} />
      ) : (
        <div className="thumb-fallback">{failed ? "no preview" : fallback || "AC"}</div>
      )}
    </div>
  );
}
