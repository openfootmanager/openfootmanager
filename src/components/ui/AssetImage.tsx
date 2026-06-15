import { useEffect, useState, type ReactNode } from "react";

interface AssetImageProps {
  src: string | null;
  alt: string;
  className?: string;
  fallback: ReactNode;
}

export default function AssetImage({
  src,
  alt,
  className = "",
  fallback,
}: AssetImageProps) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  if (!src || failed) {
    return <>{fallback}</>;
  }

  return (
    <img
      src={src}
      alt={alt}
      className={className}
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
}
