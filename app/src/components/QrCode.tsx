import { useEffect, useState } from "react";
import QRCode from "qrcode";

export function QrCode({ value, size = 148 }: { value: string; size?: number }) {
  const [src, setSrc] = useState("");

  useEffect(() => {
    if (!value) {
      setSrc("");
      return;
    }
    let cancelled = false;
    QRCode.toDataURL(value, {
      margin: 1,
      width: size * 2,
      color: { dark: "#0b0d10", light: "#ffffff" },
    })
      .then((url) => {
        if (!cancelled) setSrc(url);
      })
      .catch(() => setSrc(""));
    return () => {
      cancelled = true;
    };
  }, [value, size]);

  if (!src) return <div className="qr-placeholder" style={{ width: size, height: size }} />;
  return <img className="qr" src={src} width={size} height={size} alt="QR link server" />;
}
