interface FileSystemFileHandle {
  getFile(): Promise<File>;
}

interface FileSystemDirectoryHandle {
  getFileHandle(name: string): Promise<FileSystemFileHandle>;
}

interface Window {
  showDirectoryPicker?: () => Promise<FileSystemDirectoryHandle>;
}
