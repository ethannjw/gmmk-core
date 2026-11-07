import Foundation
import IOKit
import IOKit.hid

struct Request: Decodable {
  let op: String
  let vid: UInt32?
  let pid: UInt32?
  let usagePage: UInt32?
  let usage: UInt32?
  let data: [UInt8]?
  let reportSize: Int?
}

struct Response: Encodable {
  let ok: Bool
  let error: String?
  let product: String?
  let sent: Int?
  let data: [UInt8]?
}

func reply(_ response: Response) -> Never {
  let encoder = JSONEncoder()
  if let data = try? encoder.encode(response),
     let line = String(data: data, encoding: .utf8)
  {
    print(line)
  } else {
    print("{\"ok\":false,\"error\":\"encode failed\"}")
  }
  exit(response.ok ? 0 : 1)
}

func getFeatureReport(_ device: IOHIDDevice, reportID: CFIndex, reportSize: Int) -> [UInt8]? {
  var buffer = [UInt8](repeating: 0, count: reportSize)
  buffer[0] = UInt8(reportID)
  var length = CFIndex(reportSize)
  let result = buffer.withUnsafeMutableBytes { raw -> IOReturn in
    guard let base = raw.baseAddress else { return kIOReturnBadArgument }
    return withUnsafeMutablePointer(to: &length) { lenPtr in
      IOHIDDeviceGetReport(
        device,
        kIOHIDReportTypeFeature,
        reportID,
        base.assumingMemoryBound(to: UInt8.self),
        lenPtr
      )
    }
  }
  guard result == kIOReturnSuccess else { return nil }
  return Array(buffer.prefix(Int(length)))
}

func matchingDict(vid: UInt32, pid: UInt32, usagePage: UInt32, usage: UInt32) -> CFDictionary {
  [
    kIOHIDVendorIDKey: vid,
    kIOHIDProductIDKey: pid,
    kIOHIDPrimaryUsagePageKey: usagePage,
    kIOHIDPrimaryUsageKey: usage,
  ] as CFDictionary
}

func productName(_ device: IOHIDDevice) -> String? {
  IOHIDDeviceGetProperty(device, kIOHIDProductKey as CFString) as? String
}

func firstMatchingDevice(vid: UInt32, pid: UInt32, usagePage: UInt32, usage: UInt32) -> IOHIDDevice? {
  let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
  IOHIDManagerSetDeviceMatching(manager, matchingDict(vid: vid, pid: pid, usagePage: usagePage, usage: usage))
  guard IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone)) == kIOReturnSuccess else {
    return nil
  }
  guard let set = IOHIDManagerCopyDevices(manager) as? Set<IOHIDDevice> else {
    return nil
  }
  return set.first
}

func handle(_ req: Request) {
  let vid = req.vid ?? 0x320F
  let pid = req.pid ?? 0x504B
  let usagePage = req.usagePage ?? 0xFF01
  let usage = req.usage ?? 0x01

  switch req.op {
  case "scan":
    guard let device = firstMatchingDevice(vid: vid, pid: pid, usagePage: usagePage, usage: usage) else {
      reply(Response(ok: false, error: "Device not found", product: nil, sent: nil, data: nil))
    }
    reply(Response(ok: true, error: nil, product: productName(device), sent: nil, data: nil))

  case "getFeature":
    guard let device = firstMatchingDevice(vid: vid, pid: pid, usagePage: usagePage, usage: usage) else {
      reply(Response(ok: false, error: "Device not found", product: nil, sent: nil, data: nil))
    }
    let openResult = IOHIDDeviceOpen(device, IOOptionBits(kIOHIDOptionsTypeNone))
    guard openResult == kIOReturnSuccess else {
      reply(
        Response(
          ok: false,
          error: "IOHIDDeviceOpen failed: 0x\(String(openResult, radix: 16))",
          product: productName(device),
          sent: nil,
          data: nil
        )
      )
    }
    defer { IOHIDDeviceClose(device, IOOptionBits(kIOHIDOptionsTypeNone)) }
    let reportSize = req.reportSize ?? 264
    let reportID = CFIndex(req.data?.first ?? 7)
    guard let bytes = getFeatureReport(device, reportID: reportID, reportSize: reportSize) else {
      reply(
        Response(
          ok: false,
          error: "IOHIDDeviceGetReport failed",
          product: productName(device),
          sent: nil,
          data: nil
        )
      )
    }
    reply(Response(ok: true, error: nil, product: productName(device), sent: bytes.count, data: bytes))

  case "open":
    guard let device = firstMatchingDevice(vid: vid, pid: pid, usagePage: usagePage, usage: usage) else {
      reply(Response(ok: false, error: "Device not found", product: nil, sent: nil, data: nil))
    }
    let openResult = IOHIDDeviceOpen(device, IOOptionBits(kIOHIDOptionsTypeNone))
    guard openResult == kIOReturnSuccess else {
      reply(
        Response(
          ok: false,
          error: "IOHIDDeviceOpen failed: 0x\(String(openResult, radix: 16)). Grant Input Monitoring for native/hid-gmmk (or the packaged app).",
          product: productName(device),
          sent: nil,
          data: nil
        )
      )
    }
    defer { IOHIDDeviceClose(device, IOOptionBits(kIOHIDOptionsTypeNone)) }

    if let data = req.data, !data.isEmpty {
      let reportSize = req.reportSize ?? 264
      var buffer = data
      if buffer.count < reportSize {
        buffer.append(contentsOf: Array(repeating: 0, count: reportSize - buffer.count))
      } else if buffer.count > reportSize {
        buffer = Array(buffer.prefix(reportSize))
      }
      let reportID = CFIndex(buffer[0])
      let reportLength = buffer.count
      let setResult = buffer.withUnsafeMutableBytes { raw -> IOReturn in
        guard let base = raw.baseAddress else { return kIOReturnBadArgument }
        return IOHIDDeviceSetReport(
          device,
          kIOHIDReportTypeFeature,
          reportID,
          base.assumingMemoryBound(to: UInt8.self),
          reportLength
        )
      }
      guard setResult == kIOReturnSuccess else {
        reply(
          Response(
            ok: false,
            error: "IOHIDDeviceSetReport failed: 0x\(String(setResult, radix: 16)). Quit Glorious CORE first.",
            product: productName(device),
            sent: nil,
            data: nil
          )
        )
      }
      reply(Response(ok: true, error: nil, product: productName(device), sent: buffer.count, data: nil))
    } else {
      reply(Response(ok: true, error: nil, product: productName(device), sent: nil, data: nil))
    }

  case "setFeature":
    guard let device = firstMatchingDevice(vid: vid, pid: pid, usagePage: usagePage, usage: usage) else {
      reply(Response(ok: false, error: "Device not found", product: nil, sent: nil, data: nil))
    }
    let openResult = IOHIDDeviceOpen(device, IOOptionBits(kIOHIDOptionsTypeNone))
    guard openResult == kIOReturnSuccess else {
      reply(Response(ok: false, error: "IOHIDDeviceOpen failed", product: productName(device), sent: nil, data: nil))
    }
    defer { IOHIDDeviceClose(device, IOOptionBits(kIOHIDOptionsTypeNone)) }

    var buffer = req.data ?? []
    let reportSize = req.reportSize ?? 264
    if buffer.isEmpty {
      reply(Response(ok: false, error: "Empty report", product: nil, sent: nil, data: nil))
    }
    if buffer.count < reportSize {
      buffer.append(contentsOf: Array(repeating: 0, count: reportSize - buffer.count))
    } else if buffer.count > reportSize {
      buffer = Array(buffer.prefix(reportSize))
    }
    let reportID = CFIndex(buffer[0])
    let reportLength = buffer.count
    let setResult = buffer.withUnsafeMutableBytes { raw -> IOReturn in
      guard let base = raw.baseAddress else { return kIOReturnBadArgument }
      return IOHIDDeviceSetReport(
        device,
        kIOHIDReportTypeFeature,
        reportID,
        base.assumingMemoryBound(to: UInt8.self),
        reportLength
      )
    }
    guard setResult == kIOReturnSuccess else {
      reply(
        Response(
          ok: false,
          error: "IOHIDDeviceSetReport failed: 0x\(String(setResult, radix: 16))",
          product: productName(device),
          sent: nil,
          data: nil
        )
      )
    }
    reply(Response(ok: true, error: nil, product: productName(device), sent: buffer.count, data: nil))

  case "setThenGetFeature":
    guard let device = firstMatchingDevice(vid: vid, pid: pid, usagePage: usagePage, usage: usage) else {
      reply(Response(ok: false, error: "Device not found", product: nil, sent: nil, data: nil))
    }
    let openResult = IOHIDDeviceOpen(device, IOOptionBits(kIOHIDOptionsTypeNone))
    guard openResult == kIOReturnSuccess else {
      reply(Response(ok: false, error: "IOHIDDeviceOpen failed", product: productName(device), sent: nil, data: nil))
    }
    defer { IOHIDDeviceClose(device, IOOptionBits(kIOHIDOptionsTypeNone)) }
    var buffer = req.data ?? []
    let reportSize = req.reportSize ?? 264
    if !buffer.isEmpty {
      if buffer.count < reportSize {
        buffer.append(contentsOf: Array(repeating: 0, count: reportSize - buffer.count))
      } else if buffer.count > reportSize {
        buffer = Array(buffer.prefix(reportSize))
      }
      let reportID = CFIndex(buffer[0])
      let reportLength = buffer.count
      let setResult = buffer.withUnsafeMutableBytes { raw -> IOReturn in
        guard let base = raw.baseAddress else { return kIOReturnBadArgument }
        return IOHIDDeviceSetReport(
          device,
          kIOHIDReportTypeFeature,
          reportID,
          base.assumingMemoryBound(to: UInt8.self),
          reportLength
        )
      }
      guard setResult == kIOReturnSuccess else {
        reply(
          Response(
            ok: false,
            error: "IOHIDDeviceSetReport failed: 0x\(String(setResult, radix: 16))",
            product: productName(device),
            sent: nil,
            data: nil
          )
        )
      }
      Thread.sleep(forTimeInterval: 0.03)
    }
    let reportID = CFIndex(buffer.first ?? 7)
    guard let bytes = getFeatureReport(device, reportID: reportID, reportSize: reportSize) else {
      reply(
        Response(
          ok: false,
          error: "IOHIDDeviceGetReport failed",
          product: productName(device),
          sent: nil,
          data: nil
        )
      )
    }
    reply(Response(ok: true, error: nil, product: productName(device), sent: bytes.count, data: bytes))

  default:
    reply(Response(ok: false, error: "Unknown op: \(req.op)", product: nil, sent: nil, data: nil))
  }
}

guard CommandLine.arguments.count >= 2,
      let jsonData = CommandLine.arguments[1].data(using: .utf8),
      let req = try? JSONDecoder().decode(Request.self, from: jsonData)
else {
  reply(Response(ok: false, error: "Usage: hid-gmmk '<json>'", product: nil, sent: nil, data: nil))
}

handle(req)
