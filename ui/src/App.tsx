
import { TelemetryPanel } from './components/TelemetryPanel'
import { GraphView } from './components/GraphView'
import { CommandConsole } from './components/CommandConsole'
import { FaultPanel } from './components/FaultPanel'
import { ReplaySystem } from './components/ReplaySystem'

function App() {
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
        <div className="flex space-x-4 text-gray-500">
          <div className="w-6 h-6 border border-gray-600 rounded"></div>
          <div className="w-6 h-6 border border-gray-600 rounded-full"></div>
        </div>
      </div>

      {/* Main Grid Layout */}
      <div className="flex-1 grid grid-cols-12 gap-4 min-h-0">
        
        {/* Left Column - Commands */}
        <div className="col-span-3 flex flex-col h-full min-h-0">
          <CommandConsole />
        </div>

        {/* Center/Right Column Container */}
        <div className="col-span-9 flex flex-col h-full space-y-4 min-h-0">
          
          <div className="grid grid-cols-12 gap-4 h-3/5 min-h-0">
             <div className="col-span-9 flex flex-col h-full min-h-0">
               <TelemetryPanel />
               <GraphView />
             </div>
             
             <div className="col-span-3 flex flex-col h-full min-h-0">
               <FaultPanel />
             </div>
          </div>

          <div className="flex-1 min-h-0">
            <ReplaySystem />
          </div>

        </div>

      </div>

    </div>
  )
}

export default App
