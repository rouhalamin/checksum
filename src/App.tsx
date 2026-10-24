import { CheckSumWindow } from './components/CheckSumWindow';

export function App() {
  return (
    <div className="min-h-screen bg-[#dce1e8] flex flex-col justify-between">
      <main className="flex-1 flex flex-col items-center justify-center p-2 sm:p-4">
        <CheckSumWindow />
      </main>
    </div>
  );
}

export default App;
