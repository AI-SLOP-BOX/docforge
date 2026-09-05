import React from 'react'
import {
  SearchResult, Bookmark, FormField,
  SearchPanel, BookmarksPanel, FormsPanel, ThumbnailsPanel, InfoPanel, SeparationsPanel
} from './PDFViewerPanels'
import { PanelType } from './PDFViewerToolbar'

interface PDFViewerSidebarProps {
  activePanel: PanelType
  sepPlates: { c: boolean; m: boolean; y: boolean; k: boolean; tac: boolean }
  setSepPlates: React.Dispatch<React.SetStateAction<{ c: boolean; m: boolean; y: boolean; k: boolean; tac: boolean }>>
  tacLimit: number
  setTacLimit: (val: number) => void
  pdfData: number[] | null
  pageCount: number
  currentPage: number
  goToPage: (page: number) => void
  onPdfUpdate?: (data: number[]) => void
  searchQuery: string
  setSearchQuery: (query: string) => void
  searchResults: SearchResult[]
  handleSearch: () => void
  bookmarks: Bookmark[]
  formFields: FormField[]
  metadata: Record<string, unknown> | null
}

export const PDFViewerSidebar: React.FC<PDFViewerSidebarProps> = ({
  activePanel,
  sepPlates,
  setSepPlates,
  tacLimit,
  setTacLimit,
  pdfData,
  pageCount,
  currentPage,
  goToPage,
  onPdfUpdate,
  searchQuery,
  setSearchQuery,
  searchResults,
  handleSearch,
  bookmarks,
  formFields,
  metadata,
}) => {
  if (activePanel === 'none') return null

  return (
    <div
      style={{
        width: 280,
        background: 'var(--bg-1)',
        borderRight: '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
      }}
    >
      {activePanel === 'separations' && (
        <SeparationsPanel
          sepPlates={sepPlates}
          setSepPlates={setSepPlates}
          tacLimit={tacLimit}
          setTacLimit={setTacLimit}
        />
      )}
      {activePanel === 'thumbnails' && pdfData && (
        <ThumbnailsPanel
          pdfData={pdfData}
          pageCount={pageCount}
          currentPage={currentPage}
          onGoToPage={goToPage}
          onPdfUpdate={onPdfUpdate}
        />
      )}
      {activePanel === 'search' && (
        <SearchPanel
          query={searchQuery}
          setQuery={setSearchQuery}
          results={searchResults}
          onSearch={handleSearch}
          onGoToPage={goToPage}
        />
      )}
      {activePanel === 'bookmarks' && (
        <BookmarksPanel bookmarks={bookmarks} onGoToPage={goToPage} />
      )}
      {activePanel === 'forms' && pdfData && (
        <FormsPanel fields={formFields} pdfData={pdfData} />
      )}
      {activePanel === 'info' && (
        <InfoPanel metadata={metadata} />
      )}
    </div>
  )
}
