## IQX Dock Manager

**Real-time Insights for Optimized Shipping Performance**

The IQX Dock Manager revolutionizes warehouse shipping operations through real-time tracking, intelligent alerts, and actionable analytics. By integrating data from diverse systems, this Rust-powered solution provides insights to minimize delays, enhance throughput, and reduce costs.

**Key Features:**

* Real-time dock door, LGV, and shipment tracking
* Proactive alerts for bottlenecks and optimization opportunities
* Aggregated analytics for data-driven decision-making
* Rust backend with seamless SQL Server, MongoDB, and RabbitMQ integration
* Bridges legacy and modern systems for a holistic view

**Ideal for warehouse managers, operations teams, and IT professionals seeking to optimize shipping performance.**

## Getting Started

### Prerequisites

* Rust toolchain (installation instructions can be found at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install))
* Docker (optional, for containerized deployment)
* Access to the required databases (SQL Server, MongoDB) and RabbitMQ

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/chazwilder/iqx-dockmonitor.git
   ```

2. Navigate to the project directory:

   ```bash
   cd iqx-dockmonitor
   ```

3. Configure the application:

   * Update the configuration files (`default.yaml`, `development.yaml`, `production.yaml`, `queries.yaml`, `dock_doors.yaml`) with your specific settings.
   * Ensure the SQL queries in `queries.yaml` are compatible with your WMS database schema.

4. Build and run the application:

   ```bash
   cargo run --release
   ```

   For containerized deployment (optional):

   ```bash
   docker build -t iqx-dock-manager .
   docker run -d --name dock-manager -p 8080:8080 iqx-dock-manager
   ```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

* The Rust community for their fantastic language and ecosystem.
* The developers of the `config`, `sqlx-oldapi`, `tokio`, and other libraries used in this project.

**Disclaimer:** This project is a proof-of-concept demonstrating the power of real-time data and analytics in warehouse management. Specific configurations and integrations may need to be adapted to your environment.

**Let's revolutionize warehouse shipping together!**

**Remember to replace `your-username` with your actual GitHub username.**

This README provides a comprehensive overview of the project, installation instructions, contribution guidelines, and licensing information. It also includes acknowledgments and a disclaimer to set expectations. Feel free to enhance it further with additional sections like a project roadmap, screenshots, or a more detailed explanation of the technical architecture.

Let me know if you have any other questions or modifications! 
