import { useEffect, useState } from "react";

function detectMobilePlatform(): boolean {
  if (typeof navigator === "undefined") {
    return false;
  }

  const ua = navigator.userAgent.toLowerCase();
  return /android|iphone|ipad|ipod/.test(ua);
}

export function usePlatform() {
  const [isMobile, setIsMobile] = useState<boolean>(() => {
    if (typeof window === "undefined") {
      return false;
    }

    return detectMobilePlatform() || window.matchMedia("(max-width: 900px)").matches;
  });

  useEffect(() => {
    const media = window.matchMedia("(max-width: 900px)");

    const update = () => {
      setIsMobile(detectMobilePlatform() || media.matches);
    };

    update();
    media.addEventListener("change", update);

    return () => media.removeEventListener("change", update);
  }, []);

  return { isMobile };
}
