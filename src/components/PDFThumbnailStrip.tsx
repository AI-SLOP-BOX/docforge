import React from 'react'

interface PDFThumbnailStripProps {
  pageCount: number
  currentPage: number
  goToPage: (page: number) => void
}

export const PDFThumbnailStrip: React.FC<PDFThumbnailStripProps> = ({
  pageCount,
  currentPage,
  goToPage,
}) => {
  if (pageCount <= 0) return null

  return (
    <div
      style={{
        height: 80,
        background: 'var(--bg-1)',
        borderTop: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '0 12px',
        overflowX: 'auto',
        flexShrink: 0,
      }}
    >
      {Array.from({ length: pageCount }, (_, i) => (
        <div
          key={i}
          onClick={() => goToPage(i)}
          style={{
            width: 50,
            height: 65,
            background: 'var(--bg-2)',
            border: currentPage === i ? '2px solid var(--accent)' : '1px solid var(--border)',
            borderRadius: 4,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 10,
            color: 'var(--text-muted)',
            flexShrink: 0,
          }}
        >
          {i + 1}
        </div>
      ))}
    </div>
  )
}
