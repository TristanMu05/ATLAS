
import { TelemetryPanel } from './components/TelemetryPanel'
import { GraphView } from './components/GraphView'
import { CommandConsole } from './components/CommandConsole'
import { FaultPanel } from './components/FaultPanel'
import { ReplaySystem } from './components/ReplaySystem'
import { useTelemetry } from './contexts/TelemetryProvider'

function App() {
  const { resetSystem } = useTelemetry();

  return (
    <div className="h-screen bg-dark-900 flex flex-col p-4 overflow-hidden">
      
      {/* Top Navigation Bar */}
      <div className="flex items-center justify-between border-b border-dark-700 pb-4 mb-4">
        <div className="flex items-center space-x-3">
          <svg className="w-8 h-8 text-brand-green" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M2 20h20L12 4z"/>
          </svg>
          <h1 className="text-xl font-bold tracking-widest text-white uppercase">Atlas</h1>
          <span className="text-gray-500 font-semibold tracking-widest text-sm uppercase pl-4 border-l border-dark-700">Ground Command Center</span>
        </div>
        <div className="flex items-center space-x-3">
          <button
            onClick={resetSystem}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-gray-400 border border-dark-700 rounded hover:text-white hover:border-brand-red hover:bg-brand-red/10 transition-colors"
            title="Reset system — clear all data and set mode to IDLE"
          >
            <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h5M20 20v-5h-5M4 9a9 9 0 0115.36-5.36M20 15a9 9 0 01-15.36 5.36" />
            </svg>
            Reset
          </button>
        </div>
      </div>

      {/* Main Grid Layout */}
      <div className="flex-1 grid grid-cols-12 gap-4 min-h-0">
        
        {/* Left Column - Commands + Faults */}
        <div className="col-span-3 flex flex-col h-full min-h-0 space-y-4">
          <CommandConsole />
          <div className="flex-1 min-h-0">
            <FaultPanel />
          </div>
        </div>

        {/* Right Column - Telemetry + Graphs + Log */}
        <div className="col-span-9 flex flex-col h-full space-y-4 min-h-0">
          <TelemetryPanel />
          <div className="flex-1 min-h-0">
            <GraphView />
          </div>
          <div className="h-[300px] min-h-0">
            <ReplaySystem />
          </div>
        </div>

      </div>

    </div>
  )
}

export default App
